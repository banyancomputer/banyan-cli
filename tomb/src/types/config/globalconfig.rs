use crate::{cli::specifiers::BucketSpecifier, pipelines::error::TombError, utils::config::*};
use anyhow::{anyhow, Result};

use tomb_common::{
    banyan_api::client::{Client, Credentials},
    utils::io::get_read,
};
use tomb_crypt::prelude::*;
use uuid::Uuid;

use super::{
    bucket::LocalBucket, load_api_key, new_api_key, new_wrapping_key, save_api_key, wrapping_key,
    Endpoints,
};
use serde::{Deserialize, Serialize};
use std::{
    env::current_dir,
    fs::{remove_file, OpenOptions},
    path::{Path, PathBuf},
    str::FromStr,
};

/// Represents the Global contents of the tomb configuration file in a user's .config
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct GlobalConfig {
    /// Tomb version
    version: String,
    /// Location of wrapping key on disk in PEM format
    pub wrapping_key_path: PathBuf,
    /// Location of api key on disk in PEM format
    pub api_key_path: PathBuf,
    /// Remote endpoints
    pub endpoints: Endpoints,
    /// Remote account id
    pub remote_account_id: Option<Uuid>,
    /// Bucket Configurations
    pub(crate) buckets: Vec<LocalBucket>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            endpoints: Endpoints::default(),
            wrapping_key_path: default_wrapping_key_path(),
            api_key_path: default_api_key_path(),
            remote_account_id: None,
            buckets: Vec::new(),
        }
    }
}

// Self
impl GlobalConfig {
    async fn create() -> Result<Self> {
        // Create a default config
        let config = Self::default();
        // Create the key files referenced
        let _wrapping_key = new_wrapping_key(&config.wrapping_key_path).await?;
        let _api_key = new_api_key(&config.api_key_path).await?;
        // Ok
        Ok(config)
    }

    /// Get the wrapping key
    pub async fn wrapping_key(&self) -> Result<EcEncryptionKey> {
        wrapping_key(&self.wrapping_key_path).await
    }

    /// Get the api key
    pub async fn api_key(&self) -> Result<EcSignatureKey> {
        load_api_key(&self.api_key_path).await
    }

    // Get the Gredentials
    async fn get_credentials(&self) -> Result<Credentials> {
        if let Ok(signing_key) = self.api_key().await &&
           let Some(account_id) = self.remote_account_id {
            Ok(Credentials {
                signing_key,
                account_id
            })
        } else {
            Err(anyhow!("No credentials."))
        }
    }

    /// Get the Client data
    pub async fn get_client(&self) -> Result<Client> {
        // Create a new Client
        let mut client = Client::new(&self.endpoints.core, &self.endpoints.data)?;
        // If there are already credentials
        if let Ok(credentials) = self.get_credentials().await {
            // Set the credentials
            client.with_credentials(credentials);
        }
        // Return the Client
        Ok(client)
    }

    /// Save the Client data to the config
    pub async fn save_client(&mut self, client: Client) -> Result<()> {
        // Update the Remote endpoints
        self.endpoints.core = client.remote_core.to_string();
        self.endpoints.data = client.remote_data.to_string();
        // If there is a Claim
        if let Some(token) = client.claims {
            // Update the remote account ID
            self.remote_account_id = Some(Uuid::from_str(token.sub()?)?);
        } else {
            self.remote_account_id = None;
        }

        // If the Client has an API key
        if let Some(api_key) = client.signing_key {
            // Save the API key to disk
            save_api_key(&self.api_key_path, api_key).await?;
        }
        // Ok
        Ok(())
    }

    /// Write to disk
    pub fn to_disk(&self) -> Result<()> {
        let writer = OpenOptions::new()
            .create(true)
            .append(false)
            .truncate(true)
            .write(true)
            .open(config_path())?;

        serde_json::to_writer_pretty(writer, &self)?;
        Ok(())
    }

    /// Initialize from file on disk
    pub async fn from_disk() -> Result<Self> {
        if let Ok(file) = get_read(&config_path()) &&
           let Ok(config) = serde_json::from_reader(file) {
                Ok(config)
        } else {
            let config = Self::create().await?;
            config.to_disk()?;
            Ok(config)
        }
    }

    /// Remove a BucketConfig for an origin
    pub fn remove_bucket_by_specifier(&mut self, bucket_specifier: &BucketSpecifier) -> Result<()> {
        if let Ok(bucket) = self.get_bucket_by_specifier(bucket_specifier) {
            // Remove bucket data
            bucket.remove_data()?;
            // Find index of bucket
            let index = self
                .buckets
                .iter()
                .position(|b| *b == bucket)
                .expect("cannot find index in buckets");
            // Remove bucket config from global config
            self.buckets.remove(index);
        }
        Ok(())
    }

    /// Remove Config data associated with each Bucket
    pub fn remove_data(&self) -> Result<()> {
        // Remove bucket data
        for bucket in &self.buckets {
            bucket.remove_data()?;
        }
        // Remove global
        let path = config_path();
        if path.exists() {
            remove_file(path)?;
        }
        // Ok
        Ok(())
    }

    /// Update a given BucketConfig
    pub fn update_config(&mut self, bucket: &LocalBucket) -> Result<()> {
        // Find index
        let index = self
            .buckets
            .iter()
            .position(|b| b.origin == bucket.origin)
            .expect("cannot find index in buckets");
        // Update bucket at index
        self.buckets[index] = bucket.clone();
        // Ok
        Ok(())
    }

    /// Find a BucketConfig by origin
    pub fn get_bucket_by_origin(&self, origin: &Path) -> Option<LocalBucket> {
        self.buckets
            .clone()
            .into_iter()
            .find(|bucket| bucket.origin == origin)
    }

    /// Find a BucketConfig by origin
    pub fn get_bucket_by_remote_id(&self, id: &Uuid) -> Option<LocalBucket> {
        self.buckets
            .clone()
            .into_iter()
            .find(|bucket| bucket.remote_id == Some(*id))
    }

    /// Create a new bucket
    async fn create_bucket(&mut self, name: &str, origin: &Path) -> Result<LocalBucket> {
        let wrapping_key = self.wrapping_key().await?;
        let mut bucket = LocalBucket::new(origin, &wrapping_key).await?;
        bucket.name = name.to_string();
        self.buckets.push(bucket.clone());
        Ok(bucket)
    }

    /// Create a bucket if it doesn't exist, return the object either way
    pub async fn get_or_init_bucket(&mut self, name: &str, origin: &Path) -> Result<LocalBucket> {
        let existing = self.get_bucket_by_origin(origin);
        if let Some(config) = existing {
            Ok(config)
        } else {
            Ok(self.create_bucket(name, origin).await?)
        }
    }

    /// Get a Bucket UUID by its BucketSpecifier
    pub(crate) fn get_bucket_id(
        &self,
        bucket_specifier: &BucketSpecifier,
    ) -> Result<Uuid, TombError> {
        if let Some(id) = bucket_specifier.bucket_id {
            return Ok(id);
        }
        if let Ok(bucket) = self.get_bucket_by_specifier(bucket_specifier) && let Some(id) = bucket.remote_id {
            return Ok(id);
        }

        Err(anyhow!("bucket had no known remote").into())
    }

    pub(crate) fn get_bucket_by_specifier(
        &self,
        bucket_specifier: &BucketSpecifier,
    ) -> Result<LocalBucket, TombError> {
        // If we already have the ID and can find a bucket from it
        if let Some(id) = bucket_specifier.bucket_id && let Some(bucket) = self.get_bucket_by_remote_id(&id) {
            return Ok(bucket);
        }

        // if let Some(origin) = &bucket_specifier.origin && let Some(bucket) = self.get_bucket_by_origin(origin) {
        //     return Ok(bucket);
        // }

        // Grab an Origin
        let origin = bucket_specifier
            .origin
            .clone()
            .unwrap_or(current_dir().expect("unable to obtain current working directory"));
        // Find a BucketConfig at this origin and expect it has an ID saved as well
        if let Some(bucket) = self.get_bucket_by_origin(&origin) {
            Ok(bucket)
        } else {
            Err(TombError::unknown_path(origin))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;
    use serial_test::serial;
    use std::{fs::remove_file, path::Path};

    #[tokio::test]
    #[serial]
    async fn to_from_disk() -> Result<()> {
        // The known path of the global config file
        let known_path = config_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        let known_path = default_wrapping_key_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        let known_path = default_api_key_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        // Create default
        let original = GlobalConfig::create().await?;
        // Save to disk
        original.to_disk()?;
        // Load from disk
        let reconstructed = GlobalConfig::from_disk().await?;
        assert_eq!(original, reconstructed);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn from_disk_direct() -> Result<()> {
        // The known path of the global config file
        let known_path = config_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        let known_path = default_wrapping_key_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        let known_path = default_api_key_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        // Load from disk
        let reconstructed = GlobalConfig::from_disk().await?;
        // Assert that it is just the default config
        let known_path = default_wrapping_key_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        let known_path = default_api_key_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        assert_eq!(GlobalConfig::create().await?, reconstructed);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn add_bucket() -> Result<()> {
        // The known path of the global config file
        let known_path = config_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }

        let origin = Path::new("test");

        // Load from disk
        let mut original = GlobalConfig::from_disk().await?;
        let original_bucket = original.get_or_init_bucket("new", origin).await?;

        // Serialize to disk
        original.to_disk()?;
        let reconstructed = GlobalConfig::from_disk().await?;
        let reconstructed_bucket = reconstructed
            .get_bucket_by_origin(origin)
            .expect("bucket config does not exist for this origin");

        // Assert equality
        assert_eq!(original_bucket.metadata, reconstructed_bucket.metadata);
        assert_eq!(original_bucket.content, reconstructed_bucket.content);

        Ok(())
    }
}
