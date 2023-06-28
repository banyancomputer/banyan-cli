use crate::{
    types::blockstore::car::carv2::carv2blockstore::CarV2BlockStore,
    utils::{config::*, serialize::*},
};
use anyhow::{Ok, Result};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, remove_dir_all},
    io::{Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
};
use wnfs::private::{AesKey, PrivateDirectory, PrivateForest, TemporalKey};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BucketConfig {
    /// The name of this bucket
    bucket_name: String,
    /// The filesystem that this bucket represents
    pub(crate) origin: PathBuf,
    /// Randomly generated folder name which holds packed content and key files
    pub(crate) generated: PathBuf,
    /// roots: [
    ///     PrivateDirectory,
    ///     Metadata PrivateForest,
    ///     Content PrivateForest
    /// ]
    pub metadata: CarV2BlockStore,
    /// roots: [
    ///     Content PrivateForest
    /// ]
    pub content: CarV2BlockStore,
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
            content,
        })
    }

    pub(crate) fn remove_data(&self) -> Result<()> {
        // Remove dir if it exists
        remove_dir_all(&self.generated).ok();
        Ok(())
    }

    /// Load a TemporalKey
    pub fn get_key(&self, label: &str) -> Result<TemporalKey> {
        // The path in which we expect to find the Manifest JSON file
        let key_file = self.generated.join(format!("{}.key", label));

        // Read in the key file from the key path
        match std::fs::File::open(key_file) {
            std::result::Result::Ok(mut key_reader) => {
                // Deserialize the data read as the latest version of manifestdata
                let mut key_data: [u8; 32] = [0; 32];
                key_reader.read_exact(&mut key_data).unwrap();
                let key: TemporalKey = TemporalKey(AesKey::new(key_data));
                Ok(key)
            }
            Err(e) => Err(anyhow::Error::new(e)),
        }
    }

    /// Store a TemporalKey
    pub fn set_key(&self, temporal_key: &TemporalKey, label: &str) -> Result<()> {
        // The path in which we expect to find the Manifest JSON file
        let key_file = &self.generated.join(format!("{}.key", label));
        let mut key_writer = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(key_file)
            .expect("Failed to create key file");

        // Write the key
        key_writer.write_all(temporal_key.0.as_bytes())?;

        Ok(())
    }

    pub async fn get_all(
        &self,
    ) -> Result<(Rc<PrivateForest>, Rc<PrivateForest>, Rc<PrivateDirectory>)> {
        let key = self.get_key("root").unwrap();
        load_all(&key, &self.metadata, &self.content).await
    }

    pub async fn set_all(
        &self,
        metadata_forest: &mut Rc<PrivateForest>,
        content_forest: &mut Rc<PrivateForest>,
        root_dir: &Rc<PrivateDirectory>,
    ) -> Result<()> {
        let temporal_key = store_all(
            &self.metadata,
            &self.content,
            metadata_forest,
            content_forest,
            root_dir,
        )
        .await?;
        self.set_key(&temporal_key, "root")
    }
}
