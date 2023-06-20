use crate::{
    types::blockstore::car::carv2::carv2blockstore::CarV2BlockStore,
    utils::{
        config::*,
        disk::{key_from_disk, key_to_disk},
    },
};
use anyhow::{Ok, Result};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir, remove_dir_all, create_dir_all},
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
        // Generate a name for the generated directory
        let generated_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        // Compose the generated directory
        let generated = xdg_data_home().join(generated_name);

        // TODO (organized grime) prevent collision
        create_dir_all(&generated)?;

        Ok(Self {
            bucket_name,
            origin: origin.to_path_buf(),
            generated,
        })
    }

    pub(crate) fn remove_data(&self) -> Result<()> {
        // Remove dir if it exists
        remove_dir_all(&self.generated).ok();
        Ok(())
    }

    pub fn get_key(&self, label: &str) -> Option<TemporalKey> {
        key_from_disk(&self.generated, label).ok()
    }

    pub fn set_key(&self, temporal_key: &TemporalKey, label: &str) -> Result<()> {
        key_to_disk(&self.generated, temporal_key, label)
    }
}

// &self
impl BucketConfig {
    pub fn get_metadata(&self) -> Result<CarV2BlockStore> {
        let metadata_path = &self.generated.join("meta.car");
        println!("trying to open car file at {}", metadata_path.display());
        CarV2BlockStore::new(metadata_path)
    }

    pub fn get_content(&self) -> Result<CarV2BlockStore> {
        let metadata_path = &self.generated.join("content.car");
        CarV2BlockStore::new(metadata_path)
    }
}
