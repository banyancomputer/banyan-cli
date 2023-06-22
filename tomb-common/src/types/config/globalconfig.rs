use crate::utils::config::xdg_config_home;

use super::bucketconfig::BucketConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::{remove_file, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use wnfs::common::dagcbor;

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    version: String,
    pub remote: String,
    buckets: Vec<BucketConfig>,
}

// Self
impl GlobalConfig {
    fn get_path() -> PathBuf {
        xdg_config_home().join("global.cbor")
    }

    fn get_read() -> Result<File> {
        File::open(Self::get_path()).map_err(anyhow::Error::new)
    }

    fn get_write() -> Result<File> {
        File::create(Self::get_path()).map_err(anyhow::Error::new)
    }

    // Initialize from a reader
    pub fn from_disk() -> Result<Self> {
        let mut config_buf = Vec::new();
        match Self::get_read() {
            Ok(mut file) => {
                file.read_to_end(&mut config_buf)?;
                dagcbor::decode(&config_buf)
            }
            Err(_) => {
                Self::default().to_disk()?;
                Self::from_disk()
            }
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

    // Write to disk
    pub fn to_disk(&self) -> Result<()> {
        Self::get_write()?.write_all(&dagcbor::encode(&self)?)?;
        // println!("just wrote out globalconfig: {:?}", self);
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
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            remote: "".to_string(),
            buckets: Vec::new(),
        }
    }
}

/*


#[cfg(test)]
mod test {
    use crate::utils::{disk::*, tests::*};
    use anyhow::Result;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn disk_key() -> Result<()> {
        let test_name = "disk_key";
        // Start er up!
        let (tomb_path, global, config, metadata_forest, content_forest, dir) =
            &mut setup(test_name).await?;

        // Generate key for this directory
        let key = store_all(
            &config.metadata,
            &config.content,
            metadata_forest,
            content_forest,
            dir,
        )
        .await?;

        // Store and load
        config.set_key(&key, "root")?;
        let new_key = config.get_key("root").unwrap();

        // Assert equality
        assert_eq!(key, new_key);

        // Teardown
        teardown(test_name).await
    }

    /*

    #[tokio::test]
    #[serial]
    async fn disk_metadata() -> Result<()> {
        let test_name = "disk_metadata";
        // Setup
        let (origin, config, metadata_forest, _, root_dir) =
            &mut setup(test_name).await?;

        // Save to disk
        let key = &hot_to_disk(origin, config, metadata_forest, root_dir).await?;

        // Reload from disk
        let (new_key, _, new_metadata_forest, new_root_dir) =
            &mut hot_from_disk(&tomb_path).await?;

        // Assert equality
        assert_eq!(key, new_key);
        assert_eq!(
            metadata_forest
                .diff(new_metadata_forest, metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(root_dir, new_root_dir);

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn disk_content() -> Result<()> {
        let test_name = "disk_content";
        // Setup
        let (origin, metadata, content, metadata_forest, content_forest, root_dir) =
            &mut setup(test_name).await?;

        let config = GlobalConfig::get_bucket(&origin).unwrap();
        // Save to disk
        let key = &mut all_to_disk(
            &config,
            metadata_forest,
            content_forest,
            root_dir,
        )
        .await?;
        // Reload from disk
        let (
            new_key,
            _,
            _,
            new_metadata_forest,
            new_content_forest,
            new_root_dir,
        ) = &mut all_from_disk(&origin).await?;

        // Assert equality
        assert_eq!(key, new_key);
        // assert_eq!(manifest, new_manifest);
        assert_eq!(
            metadata_forest
                .diff(new_metadata_forest, metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(
            content_forest
                .diff(new_content_forest, content)
                .await?
                .len(),
            0
        );
        assert_eq!(root_dir, new_root_dir);

        // Teardown
        teardown(test_name).await
    }



    /// Helper function, not a test
    async fn assert_serial_all_cold(local: bool) -> Result<()> {
        let test_name: &String = &format!("serial_all_cold_{}", local);
        // Start er up!
        let (tomb_path, mut manifest, mut metadata_forest, mut content_forest, dir) =
            setup(test_name).await?;

        // Store and load
        let key = all_to_disk(
            &tomb_path,
            &mut manifest,
            &mut metadata_forest,
            &mut content_forest,
            &dir
        )
        .await?;
        let (new_key, new_manifest, new_metadata_forest, new_content_forest, new_dir) =
            all_from_disk(&tomb_path).await?;

        // Assert equality
        assert_eq!(new_key, key);
        assert_eq!(new_manifest, manifest);
        assert_eq!(
            new_metadata_forest
                .diff(&metadata_forest, &new_manifest.metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(
            new_content_forest
                .diff(&content_forest, &new_manifest.content)
                .await?
                .len(),
            0
        );
        assert_eq!(new_dir, dir);

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    async fn serial_all_content() -> Result<()> {
        assert_serial_all_cold(true).await
    }

    #[tokio::test]
    async fn serial_all_cold_remote() -> Result<()> {
        assert_serial_all_cold(false).await
    }
     */
}


 */
