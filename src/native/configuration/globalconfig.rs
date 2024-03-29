use crate::{
    api::client::{Client, Credentials},
    native::{
        configuration::{
            keys::{load_api_key, new_api_key, new_wrapping_key, save_api_key, wrapping_key},
            xdg::{config_path, default_api_key_path, default_wrapping_key_path},
        },
        sync::LocalBucket,
        NativeError,
    },
    utils::get_read,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{remove_file, OpenOptions},
    path::{Path, PathBuf},
    str::FromStr,
};
use tomb_crypt::prelude::{EcEncryptionKey, EcSignatureKey};
use url::Url;
use uuid::Uuid;

/// Represents the Global contents of the tomb configuration file in a user's .config
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct GlobalConfig {
    /// Tomb version
    version: String,
    /// Location of wrapping key on disk in PEM format
    pub wrapping_key_path: PathBuf,
    /// Location of api key on disk in PEM format
    pub api_key_path: PathBuf,
    /// Remote endpoint
    endpoint: Url,
    /// Remote account id
    remote_user_id: Option<Uuid>,
    /// Bucket Configurations
    pub(crate) buckets: Vec<LocalBucket>,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        let endpoint = Url::parse(if option_env!("DEV_ENDPOINTS").is_some() {
            "http://127.0.0.1:3001"
        } else {
            "https://beta.data.banyan.computer"
        })
        .expect("unable to parse known URLs");

        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            endpoint,
            wrapping_key_path: default_wrapping_key_path(),
            api_key_path: default_api_key_path(),
            remote_user_id: None,
            buckets: Vec::new(),
        }
    }
}

// Self
impl GlobalConfig {
    /// Create a new Global Configuration, keys, and save them all
    pub async fn new() -> Result<Self, NativeError> {
        // Create a default config
        let config = Self::default();
        config.to_disk()?;

        // Do not blindly overwrite key files if they exist
        if !config.wrapping_key_path.exists() {
            let _wrapping_key = new_wrapping_key(&config.wrapping_key_path).await?;
        }
        if !config.api_key_path.exists() {
            let _api_key = new_api_key(&config.api_key_path).await?;
        }

        // Ok
        Ok(config)
    }

    /// Get the wrapping key
    pub async fn wrapping_key(&self) -> Result<EcEncryptionKey, NativeError> {
        wrapping_key(&self.wrapping_key_path)
            .await
            .map_err(|_| NativeError::missing_wrapping_key())
    }

    /// Get the api key
    pub async fn api_key(&self) -> Result<EcSignatureKey, NativeError> {
        load_api_key(&self.api_key_path)
            .await
            .map_err(|_| NativeError::missing_api_key())
    }

    // Get the Gredentials
    async fn get_credentials(&self) -> Result<Credentials, NativeError> {
        Ok(Credentials {
            user_id: self.remote_user_id.ok_or(NativeError::missing_user_id())?,
            signing_key: self.api_key().await?,
        })
    }

    /// Get the Client data
    pub async fn get_client(&self) -> Result<Client, NativeError> {
        // Create a new Client
        let mut client = Client::new(self.endpoint.as_ref())?;
        // If there are already credentials
        if let Ok(credentials) = self.get_credentials().await {
            // Set the credentials
            client.with_credentials(credentials);
        }
        // Return the Client
        Ok(client)
    }

    #[allow(unused)]
    /// Save the Client data to the config
    pub async fn save_client(&mut self, client: Client) -> Result<(), NativeError> {
        // Update the Remote endpoints
        self.endpoint = client.remote_core;
        // If there is a Claim
        if let Some(token) = client.claims {
            // Update the remote account ID
            self.remote_user_id =
                Some(Uuid::from_str(token.sub()?).map_err(|_| NativeError::bad_data())?);
        }

        // If the Client has an API key
        if let Some(api_key) = client.signing_key {
            // Save the API key to disk
            save_api_key(&self.api_key_path, api_key).await?;
        }

        self.to_disk()
    }

    #[allow(unused)]
    pub fn get_endpoint(&self) -> Url {
        self.endpoint.clone()
    }

    pub fn set_endpoint(&mut self, endpoint: Url) -> Result<(), NativeError> {
        self.endpoint = endpoint;
        self.to_disk()
    }

    /// Write to disk
    fn to_disk(&self) -> Result<(), NativeError> {
        let writer = OpenOptions::new()
            .create(true)
            .append(false)
            .truncate(true)
            .write(true)
            .open(config_path())?;

        serde_json::to_writer_pretty(writer, &self).map_err(|_| NativeError::bad_data())
    }

    /// Initialize from file on disk
    pub async fn from_disk() -> Result<Self, NativeError> {
        let file = get_read(&config_path())?;
        let config = serde_json::from_reader(file).map_err(|_| NativeError::bad_data())?;
        Ok(config)
    }

    /// Remove a BucketConfig for an origin
    pub fn remove_bucket(&mut self, bucket: &LocalBucket) -> Result<(), NativeError> {
        // Remove bucket data
        bucket.remove_data()?;
        // Find index of bucket
        let index = self
            .buckets
            .iter()
            .position(|b| b == bucket)
            .expect("cannot find index in buckets");
        // Remove bucket config from global config
        self.buckets.remove(index);
        self.to_disk()
    }

    /// Remove Config data associated with each Bucket
    pub fn remove_all_data(&self) -> Result<(), NativeError> {
        // Remove bucket data
        for bucket in &self.buckets {
            bucket.remove_data()?;
        }
        // Remove global
        let path = config_path();
        if path.exists() {
            remove_file(path)?;
        }
        self.to_disk()
    }

    /// Update a given BucketConfig
    pub fn update_config(&mut self, bucket: &LocalBucket) -> Result<(), NativeError> {
        // Find index
        let index = self
            .buckets
            .iter()
            .position(|b| b.origin == bucket.origin)
            .ok_or(NativeError::missing_local_drive())?;
        // Update bucket at index
        self.buckets[index] = bucket.clone();
        self.to_disk()
    }

    /// Create a new bucket
    async fn create_bucket(
        &mut self,
        name: &str,
        origin: &Path,
    ) -> Result<LocalBucket, NativeError> {
        let wrapping_key = self.wrapping_key().await?;
        let mut bucket = LocalBucket::new(origin, &wrapping_key).await?;
        bucket.name = name.to_string();
        self.buckets.push(bucket.clone());
        self.to_disk()?;
        Ok(bucket)
    }

    /// Get a Bucket configuration by the origin
    pub fn get_bucket(&self, origin: &Path) -> Option<LocalBucket> {
        self.buckets
            .iter()
            .find(|bucket| bucket.origin == origin)
            .cloned()
    }

    /// Create a bucket if it doesn't exist, return the object either way
    pub async fn get_or_init_bucket(
        &mut self,
        name: &str,
        origin: &Path,
    ) -> Result<LocalBucket, NativeError> {
        if let Some(config) = self.get_bucket(origin) {
            Ok(config.clone())
        } else {
            Ok(self.create_bucket(name, origin).await?)
        }
    }
}

#[cfg(test)]
mod test {

    use serial_test::serial;
    use std::{fs::remove_file, path::Path};

    use crate::native::{
        configuration::{
            globalconfig::GlobalConfig,
            xdg::{config_path, default_api_key_path, default_wrapping_key_path},
        },
        NativeError,
    };

    #[tokio::test]
    #[serial]
    async fn to_from_disk() -> Result<(), NativeError> {
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
        let original = GlobalConfig::new().await?;
        // Load from disk
        let reconstructed = GlobalConfig::from_disk().await?;
        assert_eq!(original, reconstructed);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn from_disk_direct() -> Result<(), NativeError> {
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
        let reconstructed = GlobalConfig::new().await?;
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
        assert_eq!(GlobalConfig::from_disk().await?, reconstructed);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn add_bucket() -> Result<(), NativeError> {
        // The known path of the global config file
        let known_path = config_path();
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }

        let origin = Path::new("test");

        // Create
        let mut original = GlobalConfig::new().await?;
        let original_bucket = original.get_or_init_bucket("new", origin).await?;
        // Save
        original.to_disk()?;
        let reconstructed = GlobalConfig::from_disk().await?;
        let reconstructed_bucket = reconstructed
            .get_bucket(origin)
            .expect("bucket config does not exist for this origin");

        // Assert equality
        assert_eq!(original_bucket.metadata, reconstructed_bucket.metadata);
        assert_eq!(original_bucket.content, reconstructed_bucket.content);

        Ok(())
    }
}
