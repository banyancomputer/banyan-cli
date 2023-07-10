use crate::utils::config::xdg_config_home;
use anyhow::Result;
use tomb_common::crypto::rsa::RsaPrivateKey;

use super::bucketconfig::BucketConfig;
use serde::{Deserialize, Serialize};
use std::{
    fs::{remove_file, File},
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GlobalConfig {
    version: String,
    pub wrapping_key_path: PathBuf,
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

    // Write to disk
    pub fn to_disk(&self) -> Result<()> {
        serde_json::to_writer_pretty(Self::get_write()?, &self)?;
        Ok(())
    }

    // Initialize from a reader
    pub fn from_disk() -> Result<Self> {
        if let Ok(file) = Self::get_read() &&
           let Ok(config) = serde_json::from_reader(file) {
                Ok(config)
        } else {
            Self::default()?.to_disk()?;
            Self::from_disk()
        }
    }

    pub fn get_bucket(&self, origin: &Path) -> Option<BucketConfig> {
        self.find_config(origin)
    }

    pub fn new_bucket(&mut self, origin: &Path) -> Result<BucketConfig> {
        self.find_or_create_config(origin)
    }

    pub fn remove(&mut self, origin: &Path) -> Result<()> {
        if let Some(bucket) = self.find_config(origin) {
            // Remove bucket data
            bucket.remove_data()?;
            // Find index of bucket
            let index = self.buckets.iter().position(|b| *b == bucket).unwrap();
            // Remove bucket config from global config
            self.buckets.remove(index);
        }
        Ok(())
    }

    pub fn remove_data(&self) -> Result<()> {
        // Remove bucket data
        for bucket in &self.buckets {
            bucket.remove_data()?;
        }
        // Remove global
        remove_file(Self::get_path()).ok();
        // Ok
        Ok(())
    }

    pub fn update_config(&mut self, bucket: &BucketConfig) -> Result<()> {
        // Find index
        let index = self
            .buckets
            .iter()
            .position(|b| b.origin == bucket.origin)
            .unwrap();
        // Update bucket at index
        self.buckets[index] = bucket.clone();
        // Ok
        Ok(())
    }

    fn find_config(&self, path: &Path) -> Option<BucketConfig> {
        self.buckets
            .clone()
            .into_iter()
            .find(|bucket| bucket.origin == path)
    }

    fn create_config(&mut self, origin: &Path) -> Result<BucketConfig> {
        let bucket = BucketConfig::new(origin)?;
        self.buckets.push(bucket.clone());
        Ok(bucket)
    }

    fn find_or_create_config(&mut self, path: &Path) -> Result<BucketConfig> {
        let existing = self.find_config(path);
        if let Some(config) = existing {
            Ok(config)
        } else {
            Ok(self.create_config(path)?)
        }
    }

    fn default() -> Result<Self> {
        // Path of the wrapping_key file
        let wrapping_key_path = xdg_config_home().join("wrapping_key.pem");
        // Load if it already exists
        let wrapping_key = if wrapping_key_path.exists() {
            RsaPrivateKey::from_pem_file(&wrapping_key_path)?
        } else {
            RsaPrivateKey::new()?
        };

        // Save the key to disk
        wrapping_key.to_pem_file(&wrapping_key_path)?;

        // Create new Global Config
        Ok(Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            remote: "".to_string(),
            wrapping_key_path,
            buckets: Vec::new(),
        })
    }

    pub fn wrapping_key_from_disk(&self) -> Result<RsaPrivateKey> {
        RsaPrivateKey::from_pem_file(&self.wrapping_key_path)
    }

    pub fn wrapping_key_to_disk(&self, wrapping_key: &RsaPrivateKey) -> Result<()> {
        wrapping_key.to_pem_file(&self.wrapping_key_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{types::config::globalconfig::GlobalConfig, utils::config::xdg_config_home};
    use anyhow::Result;
    use serial_test::serial;
    use std::{fs::remove_file, path::Path};

    #[test]
    #[serial]
    fn to_from_disk() -> Result<()> {
        // The known path of the global config file
        let known_path = xdg_config_home().join("global.json");
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        // Create default
        let original = GlobalConfig::default()?;
        // Save to disk
        original.to_disk()?;
        // Load from disk
        let reconstructed = GlobalConfig::from_disk()?;
        assert_eq!(original, reconstructed);
        Ok(())
    }

    #[test]
    #[serial]
    fn from_disk_direct() -> Result<()> {
        // The known path of the global config file
        let known_path = xdg_config_home().join("global.json");
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }
        // Load from disk
        let reconstructed = GlobalConfig::from_disk()?;
        // Assert that it is just the default config
        assert_eq!(GlobalConfig::default()?, reconstructed);
        Ok(())
    }

    #[test]
    #[serial]
    #[ignore]
    fn add_bucket() -> Result<()> {
        // The known path of the global config file
        let known_path = xdg_config_home().join("global.json");
        // Remove it if it exists
        if known_path.exists() {
            remove_file(&known_path)?;
        }

        let origin = Path::new("test");

        // Load from disk
        let mut original = GlobalConfig::from_disk()?;
        let original_bucket = original.new_bucket(origin)?;

        // Serialize to disk
        original.to_disk()?;
        let reconstructed = GlobalConfig::from_disk()?;
        let reconstructed_bucket = reconstructed.get_bucket(origin).unwrap();

        // Assert equality
        assert_eq!(original_bucket.metadata, reconstructed_bucket.metadata);
        assert_eq!(original_bucket.content.deltas[0], reconstructed_bucket.content.deltas[0]);

        Ok(())
    }

    /*
    #[tokio::test]
    #[serial]
    #[ignore]
    async fn get_set_all() -> Result<()> {
        let test_name = "get_set_key";
        // Start er up!
        let (origin, global, config, metadata_forest, content_forest, dir) =
            &mut setup(test_name).await?;

        config
            .set_all(metadata_forest, content_forest, &dir)
            .await?;
        global.update_config(config)?;
        global.to_disk()?;

        let new_global = GlobalConfig::from_disk()?;
        let new_config = &mut new_global.get_bucket(origin).unwrap();

        assert_eq!(config.origin, new_config.origin);
        assert_eq!(config.generated, new_config.generated);
        assert_eq!(config.metadata.car.header, new_config.metadata.car.header);
        assert_eq!(config.metadata.car.index, new_config.metadata.car.index);
        assert_eq!(
            config.metadata.car.car.header,
            new_config.metadata.car.car.header
        );
        assert_eq!(
            config.metadata.car.car.index,
            new_config.metadata.car.car.index
        );
        assert_eq!(config, new_config);

        let (new_metadata_forest, new_content_forest, new_dir) = &new_config.get_all().await?;

        // Assert equality
        assert_eq!(
            metadata_forest
                .diff(new_metadata_forest, &new_config.metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(
            content_forest
                .diff(new_content_forest, &new_config.content)
                .await?
                .len(),
            0
        );
        assert_eq!(dir, new_dir);

        // Teardown
        teardown(test_name).await
    }
     */
}
