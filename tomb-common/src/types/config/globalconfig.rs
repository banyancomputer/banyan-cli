use crate::{utils::config::xdg_config_home, types::blockstore::car::carv2::carv2blockstore::CarV2BlockStore};

use super::bucketconfig::BucketConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::{remove_file, File},
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    version: String,
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

    // Initialize from a reader
    pub fn from_disk() -> Result<Self> {
        match Self::get_read() {
            Ok(file) => Ok(serde_json::from_reader::<File, Self>(file)?),
            Err(_) => {
                Self::default().to_disk()?;
                Self::from_disk()
            }
        }
    }

    pub fn get_bucket(&self, origin: &Path) -> Result<BucketConfig> {
        let mut config = self.find_config(origin).unwrap();
        config.metadata = CarV2BlockStore::new(&config.metadata.path)?;
        config.content = CarV2BlockStore::new(&config.content.path)?;
        Ok(config)
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
        serde_json::to_writer_pretty(Self::get_write()?, &self)?;
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


#[cfg(test)]
mod test {
    use crate::{utils::tests::*, types::config::globalconfig::GlobalConfig};
    use anyhow::Result;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn get_set_all() -> Result<()> {
        let test_name = "get_set_key";
        // Start er up!
        let (origin, global, config, metadata_forest, content_forest, dir) =
            &mut setup(test_name).await?;

        config.set_all_metadata(metadata_forest, content_forest, &dir).await?;
        global.update_config(config)?;
        global.to_disk()?;

        let new_global = GlobalConfig::from_disk()?;
        let new_config = &mut new_global.get_bucket(origin).unwrap();

        assert_eq!(config.origin, new_config.origin);
        assert_eq!(config.generated, new_config.generated);
        assert_eq!(config.metadata.carv2.header, new_config.metadata.carv2.header);
        assert_eq!(config.metadata.carv2.index, new_config.metadata.carv2.index);
        assert_eq!(config.metadata.carv2.carv1.header, new_config.metadata.carv2.carv1.header);
        assert_eq!(config.metadata.carv2.carv1.index, new_config.metadata.carv2.carv1.index);
        assert_eq!(config, new_config);

        let (new_metadata_forest, new_content_forest, new_dir) = &new_config.get_all_metadata().await?;

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
}