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
    fn from_disk() -> Result<Self> {
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

    pub fn get_bucket(origin: &Path) -> Option<BucketConfig> {
        Self::from_disk().unwrap().find_config(origin)
    }

    pub fn new_bucket(origin: &Path) -> Result<BucketConfig> {
        let mut config = Self::from_disk().unwrap();
        let bucket = config.find_or_create_config(origin)?;
        config.to_disk().unwrap();
        Ok(bucket)
    }

    pub fn remove(origin: &Path) -> Result<()> {
        let mut config = Self::from_disk()?;
        if let Some(bucket) = config.find_config(origin) {
            // Remove bucket data
            bucket.remove_data()?;
            // Find index of bucket
            let index = config.buckets.iter().position(|b| *b == bucket).unwrap();
            // Remove bucket config from global config
            config.buckets.remove(index);
            // Rewrite global config
            config.to_disk()?
        }
        Ok(())
    }
}

// &self
impl GlobalConfig {
    // Write to disk
    pub fn to_disk(&self) -> Result<()> {
        Self::get_write()?.write_all(&dagcbor::encode(&self)?)?;
        Ok(())
    }

    fn find_config(&self, path: &Path) -> Option<BucketConfig> {
        self.buckets
            .clone()
            .into_iter()
            .find(|bucket| bucket.origin == path)
    }

    fn create_config(&mut self, origin: &Path) -> Result<BucketConfig> {
        let config = BucketConfig::new(origin)?;
        self.buckets.push(config.clone());
        self.to_disk()?;
        Ok(config)
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
