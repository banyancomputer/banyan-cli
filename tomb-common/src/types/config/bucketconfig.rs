use crate::{
    types::blockstore::car::carv2::carv2blockstore::CarV2BlockStore,
    utils::{config::*, disk::key_from_disk},
};
use anyhow::{Ok, Result};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::create_dir,
    path::{Path, PathBuf},
};
use wnfs::private::TemporalKey;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BucketConfig {
    /// The name of this bucket
    bucket_name: String,
    /// The filesystem that this bucket represents
    pub(crate) origin: PathBuf,
    /// Randomly generated folder name which holds packed content and key files
    generated: PathBuf,
}

// Self
impl BucketConfig {
    pub fn new(origin: &Path) -> Result<Self> {
        let bucket_name = origin.file_name().unwrap().to_str().unwrap().to_string();

        let generated_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();

        let generated = xdg_data_home().join(generated_name);

        create_dir(&generated)?;

        Ok(Self {
            bucket_name,
            origin: origin.to_path_buf(),
            generated,
        })
    }

    pub fn get_key(&self) -> Option<TemporalKey> {
        key_from_disk(&self.generated, "root").ok()
    }
}

// &self
impl BucketConfig {
    pub fn get_metadata(&self) -> Result<CarV2BlockStore> {
        let metadata_path = &self.generated.join("meta.car");
        CarV2BlockStore::new(metadata_path)
    }

    pub fn get_content(&self) -> Result<CarV2BlockStore> {
        let metadata_path = &self.generated.join("content.car");
        CarV2BlockStore::new(metadata_path)
    }
}
