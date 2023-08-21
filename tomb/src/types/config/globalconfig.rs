use crate::utils::config::xdg_config_home;
use anyhow::Result;
use async_recursion::async_recursion;
use tomb_crypt::prelude::*;

use super::bucketconfig::BucketConfig;
use serde::{Deserialize, Serialize};
use std::{
    fs::{remove_file, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// Represents the Global contents of the tomb configuration file in a user's .config
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GlobalConfig {
    version: String,
    /// Location of PEM key used to encrypt / decrypt
    pub wrapping_key_path: PathBuf,
    /// Remote endpoint for Metadata API
    pub remote: String,
    buckets: Vec<BucketConfig>,
}

// Self
impl GlobalConfig {
    fn get_path() -> PathBuf {
        xdg_config_home().join("global.json")
    }

    fn get_read() -> Result<File> {
        File::open(Self::get_path()).map_err(anyhow::Error::new)
    }

    fn get_write() -> Result<File> {
        File::create(Self::get_path()).map_err(anyhow::Error::new)
    }

    /// Write to disk
    pub fn to_disk(&self) -> Result<()> {
        serde_json::to_writer_pretty(Self::get_write()?, &self)?;
        Ok(())
    }

    /// Initialize from a reader
    #[async_recursion(?Send)]
    pub async fn from_disk() -> Result<Self> {
        println!("doing the from-disk Global config!");
        if let Ok(file) = Self::get_read() &&
           let Ok(config) = serde_json::from_reader(file) {
            println!("found an existing config, returning it");
                Ok(config)
        } else {
            println!("creating a default to serialize");
            Self::default().await?.to_disk()?;
            let r = Self::from_disk().await;
            println!("successfully started from scratch");
            r
        }
    }

    /// Create a new BucketConfig for an origin
    pub fn new_bucket(&mut self, origin: &Path) -> Result<BucketConfig> {
        self.find_or_create_config(origin)
    }

    /// Remove a BucketConfig for an origin
    pub fn remove(&mut self, origin: &Path) -> Result<()> {
        if let Some(bucket) = self.get_bucket(origin) {
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
        let path = Self::get_path();
        if path.exists() {
            remove_file(path)?;
        }
        // Ok
        Ok(())
    }

    /// Update a given BucketConfig
    pub fn update_config(&mut self, bucket: &BucketConfig) -> Result<()> {
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
    pub fn get_bucket(&self, origin: &Path) -> Option<BucketConfig> {
        self.buckets
            .clone()
            .into_iter()
            .find(|bucket| bucket.origin == origin)
    }

    fn create_config(&mut self, origin: &Path) -> Result<BucketConfig> {
        let bucket = BucketConfig::new(origin)?;
        self.buckets.push(bucket.clone());
        Ok(bucket)
    }

    pub(crate) fn find_or_create_config(&mut self, path: &Path) -> Result<BucketConfig> {
        let existing = self.get_bucket(path);
        if let Some(config) = existing {
            Ok(config)
        } else {
            Ok(self.create_config(path)?)
        }
    }

    async fn default() -> Result<Self> {
        // Path of the wrapping_key file
        let wrapping_key_path = xdg_config_home().join("wrapping_key.pem");
        // Load if it already exists
        let wrapping_key = if wrapping_key_path.exists() {
            Self::wrapping_key_from_disk(&wrapping_key_path).await?
        } else {
            EcEncryptionKey::generate().await?
        };

        // Save the key to disk
        Self::wrapping_key_to_disk(&wrapping_key_path, &wrapping_key).await?;

        // Create new Global Config
        Ok(Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            remote: "".to_string(),
            wrapping_key_path,
            buckets: Vec::new(),
        })
    }

    /// Load the wrapping key from disk with the known wrapping key path
    pub async fn load_key(&self) -> Result<EcEncryptionKey> {
        Self::wrapping_key_from_disk(&self.wrapping_key_path).await
    }

    /// Load the WrappingKey from its predetermined location
    async fn wrapping_key_from_disk(path: &Path) -> Result<EcEncryptionKey> {
        let mut pem_bytes = Vec::new();
        println!("opening key!");
        let mut file = File::open(path)?;
        file.read_to_end(&mut pem_bytes)?;
        println!("read key!");
        // Return
        Ok(EcEncryptionKey::import(&pem_bytes)
            .await
            .expect("Unable to convert PEM bytes to Key"))
    }

    /// Write the WRappingKey to its predetermined location
    async fn wrapping_key_to_disk(path: &Path, wrapping_key: &EcEncryptionKey) -> Result<()> {
        // PEM
        let pem_bytes = wrapping_key.export().await?;
        let mut file = File::create(path)?;
        file.write_all(&pem_bytes)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{types::config::globalconfig::GlobalConfig, utils::config::xdg_config_home};
    use anyhow::Result;
    use serial_test::serial;
    use std::{fs::remove_file, path::Path};

    #[tokio::test]
    #[serial]
    async fn to_from_disk() -> Result<()> {
        // The known path of the global config file
        let known_path = xdg_config_home().join("global.json");
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        // Create default
        let original = GlobalConfig::default().await?;
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
        let known_path = xdg_config_home().join("global.json");
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        // Load from disk
        let reconstructed = GlobalConfig::from_disk().await?;
        // Assert that it is just the default config
        assert_eq!(GlobalConfig::default().await?, reconstructed);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn add_bucket() -> Result<()> {
        // The known path of the global config file
        let known_path = xdg_config_home().join("global.json");
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }

        let origin = Path::new("test");

        // Load from disk
        let mut original = GlobalConfig::from_disk().await?;
        let original_bucket = original.new_bucket(origin)?;

        // Serialize to disk
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
