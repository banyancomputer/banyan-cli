use crate::utils::config::xdg_config_home;

use super::bucketconfig::BucketConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
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
        println!("just wrote out globalconfig: {:?}", self);
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
