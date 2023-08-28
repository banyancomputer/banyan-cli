use anyhow::{Ok, Result};
use chrono::Utc;
use rand::{distributions::Alphanumeric, Rng};
use std::{
    convert::TryFrom,
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
    rc::Rc,
};
use tomb_common::{
    blockstore::{
        carv2_disk::CarV2DiskBlockStore, multi_carv2_disk::MultiCarV2DiskBlockStore, BlockStore,
        RootedBlockStore,
    },
    metadata::FsMetadata
};
use tomb_crypt::prelude::*;
use wnfs::{
    common::Metadata,
    libipld::Cid,
    private::{PrivateDirectory, PrivateForest, PrivateNodeOnPathHistory},
};

use crate::types::config::BucketConfig;

/// Configuration for an individual Bucket / FileSystem
#[derive(Debug)]
pub struct BucketMount<MBS: RootedBlockStore, CBS: RootedBlockStore> {
    /// Encrypted metadata
    metadata: MBS,
    /// Encrypted Content
    content: CBS,
    /// Fs Metadata
    fs_metadata: FsMetadata 
}

impl BucketMount<CarV2DiskBlockStore, MultiCarV2DiskBlockStore> {
    /// Initialize a new BucketMount from a config and a wrapping key
    pub async fn init(config: &BucketConfig, wrapping_key: &EcEncryptionKey) -> Result<Self> {
        let metadata = CarV2DiskBlockStore::new(&config.metadata_path)?;
        let mut content = MultiCarV2DiskBlockStore::new(&config.content_path)?;
        content.add_delta()?;
        // Initialize the fs metadata
        let mut fs_metadata = FsMetadata::init(wrapping_key).await?;
        // Save our fs metadata in both of our stores
        fs_metadata.save(&metadata, &content).await?;
        let mut ret = Self {
            metadata,
            content,
            fs_metadata 
        };
        ret.mkdir(PathBuf::from("/")).await?;
        Ok(ret)
    }

    /// Mount a bucket from its config and a wrapping key
    pub async fn mount(config: BucketConfig, wrapping_key: &EcEncryptionKey) -> Result<Self> {
        let metadata = CarV2DiskBlockStore::new(&config.metadata_path)?;
        let content = MultiCarV2DiskBlockStore::new(&config.content_path)?;
        let fs_metadata = FsMetadata::unlock(wrapping_key, &metadata).await?;
        Ok(Self {
            metadata,
            content,
            fs_metadata 
        })
    }

    /// Mkdir a directory
    /// # Arguments
    /// * `path` - The path to mkdir
    pub async fn mkdir(&mut self, path: PathBuf) -> Result<()> {
        println!("mkdir: {:?}", path);
        // let path_segments = path.split('/').collect::<Vec<&str>>();
        let path_segments: Vec<String> = path
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        println!("path_segments: {:?}", path_segments);
        let _ = self.fs_metadata
            .root_dir
            .mkdir(
                path_segments.as_slice(),
                true,
                Utc::now(),
                &self.fs_metadata.metadata_forest,
                &self.metadata,
                &mut rand::thread_rng(),

            )
            .await
            .expect("could not mkdir");
        Ok(())
    }

    /// Ls the root directory of the bucket
    /// # Arguments
    /// * `path` - The path to ls
    /// # Returns
    /// Vec<(String, Metadata)> - A vector of entries in the directory
    pub async fn ls(self, path: PathBuf) -> Result<Vec<String>> {
        println!("ls: {:?}", path);
        // let path_segments = path.split('/').collect::<Vec<&str>>();
        let path_segments: Vec<String> = path
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect();
        let metadata_forest = self.fs_metadata.metadata_forest.as_ref();
        println!("path_segments: {:?}", path_segments);
        let entries = &self.fs_metadata
            .root_dir
            .ls(
                path_segments.as_slice(),
                true,
                metadata_forest,
                &self.metadata,
            )
            .await
            .expect("could not ls");
        println!("entries: {:?}", entries);
        let entries = entries
            .iter()
            .map(|(name, _entry)| name.to_string())
            .collect::<Vec<_>>();
        Ok(entries)
    }
}
