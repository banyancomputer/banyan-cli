use crate::{
    types::blockstore::car::carv2::carv2blockstore::CarV2BlockStore,
    utils::config::*,
};
use anyhow::{Ok, Result};
use log::info;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir, create_dir_all, remove_dir_all},
    path::{Path, PathBuf}, io::{Write, Read},
};
use wnfs::private::{TemporalKey, AesKey};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct BucketConfig {
    /// The name of this bucket
    bucket_name: String,
    /// The filesystem that this bucket represents
    pub(crate) origin: PathBuf,
    /// Randomly generated folder name which holds packed content and key files
    generated: PathBuf,
    pub(crate) metadata: CarV2BlockStore,
    pub(crate) content: CarV2BlockStore
}

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

        let metadata = CarV2BlockStore::new(&generated.join("meta.car"))?;
        let content = CarV2BlockStore::new(&generated.join("content.car"))?;

        Ok(Self {
            bucket_name,
            origin: origin.to_path_buf(),
            generated,
            metadata,
            content
        })
    }

    pub(crate) fn remove_data(&self) -> Result<()> {
        // Remove dir if it exists
        remove_dir_all(&self.generated).ok();
        Ok(())
    }

    /// Load a TemporalKey
    pub fn get_key(&self, label: &str) -> Option<TemporalKey> {
        info!("Loading in {} Key from disk", label);
        // The path in which we expect to find the Manifest JSON file
        let key_file = self.generated.join(format!("{}.key", label));

        // Read in the key file from the key path
        let mut key_reader = std::fs::File::open(key_file)
            .map_err(|e| anyhow::anyhow!("Failed to open key file: {}", e)).unwrap();
        // Deserialize the data read as the latest version of manifestdata
        let mut key_data: [u8; 32] = [0; 32];
        key_reader.read_exact(&mut key_data).unwrap();
        let key: TemporalKey = TemporalKey(AesKey::new(key_data));

        Some(key)
    }

    /// Store a TemporalKey
    pub fn set_key(&self, temporal_key: &TemporalKey, label: &str) -> Result<()> {
        // The path in which we expect to find the Manifest JSON file
        let key_file = &self.generated.join(format!("{}.key", label));
        let mut key_writer = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&key_file)
            .expect(&format!("Failed to create key file at {}", key_file.display()));

        // Write the key
        key_writer.write_all(temporal_key.0.as_bytes())?;

        Ok(())
    }
}

// impl Serialize for BucketConfig {
//     fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
//     where
//         S: serde::Serializer {
//         todo!()
//     }
// }

// impl<'de> Deserialize<'de> for BucketConfig {
//     fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de> {
//         todo!()
//     }
// }