use anyhow::{Ok, Result};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
};
use tomb_common::{
    banyan_api::models::metadata::{Metadata, MetadataState},
    blockstore::{
        carv2_disk::CarV2DiskBlockStore, multi_carv2_disk::MultiCarV2DiskBlockStore,
        RootedBlockStore,
    },
    metadata::FsMetadata,
};
use tomb_crypt::prelude::*;
use uuid::Uuid;
use wnfs::private::PrivateNodeOnPathHistory;

use crate::utils::{config::xdg_data_home, wnfsio::compute_directory_size};

const BUCKET_METADATA_FILE_NAME: &str = "metadata.car";
const BUCKET_CONTENT_DIR_NAME: &str = "deltas";

fn bucket_data_home(local_id: &str) -> PathBuf {
    xdg_data_home().join(local_id)
}

fn bucket_metadata_path(name: &str) -> PathBuf {
    xdg_data_home().join(name).join(BUCKET_METADATA_FILE_NAME)
}

fn bucket_content_path(name: &str) -> PathBuf {
    let path = xdg_data_home().join(name).join(BUCKET_CONTENT_DIR_NAME);
    // If the directory doesnt exist yet, make it!
    if !path.exists() {
        create_dir_all(&path).expect("failed to create XDG data home");
    }
    path
}

// TODO: This is maybe better concieved of as a Bucket
/// Configuration for an individual Bucket / FileSystem
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BucketConfig {
    /// The name of this bucket
    pub(crate) name: String,
    /// The filesystem that this bucket represents
    pub(crate) origin: PathBuf,
    /// Randomly generated folder name which holds bundled content and key files
    pub(crate) local_id: String,
    /// Bucket Uuid, if this
    pub(crate) remote_id: Option<Uuid>,
    /// BlockStore for storing metadata only
    pub metadata: CarV2DiskBlockStore,
    /// BlockStore for storing metadata and file content
    pub content: MultiCarV2DiskBlockStore,
}

impl Display for BucketConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "\n| LOCAL BUCKET INFO |\nname:\t\t{}\nlocal_path:\t{}\nlocal_id:\t{}\nremote_id:\t{:?}",
            self.name,
            self.origin.display(),
            self.local_id,
            self.remote_id
        ))
    }
}

impl BucketConfig {
    /// Given a directory, initialize a configuration for it
    pub async fn new(origin: &Path, wrapping_key: &EcEncryptionKey) -> Result<Self> {
        let name = origin
            .file_name()
            .expect("no file name")
            .to_str()
            .expect("no file name str")
            .to_string();
        // Generate a name for the generated directory
        let local_id: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        // Compose the generated directory
        let metadata_path = bucket_metadata_path(&local_id);
        let content_path = bucket_content_path(&local_id);
        let metadata = CarV2DiskBlockStore::new(&metadata_path)?;
        let content = MultiCarV2DiskBlockStore::new(&content_path)?;

        // Initialize the fs metadata
        let mut fs_metadata = FsMetadata::init(wrapping_key).await?;
        let public_key = wrapping_key.public_key()?;

        // Save our fs to establish map
        fs_metadata.save(&metadata, &metadata).await?;
        // Share it with the owner of this wrapping key
        fs_metadata.share_with(&public_key, &metadata).await?;
        // Save our fs again
        fs_metadata.save(&metadata, &metadata).await?;

        Ok(Self {
            name,
            origin: origin.to_path_buf(),
            local_id,
            remote_id: None,
            metadata,
            content,
        })
    }

    pub(crate) fn remove_data(&self) -> Result<()> {
        // Remove dir if it exists
        if bucket_data_home(&self.local_id).exists() {
            remove_dir_all(bucket_data_home(&self.local_id))?;
        }
        Ok(())
    }

    ///
    pub async fn unlock_fs(&self, wrapping_key: &EcEncryptionKey) -> Result<FsMetadata> {
        FsMetadata::unlock(wrapping_key, &self.metadata).await
    }

    /// Shortcut for saving a filesystem
    pub async fn save_fs(&self, fs: &mut FsMetadata) -> Result<()> {
        fs.save(&self.metadata, &self.content).await
    }

    /// Shortcut for serialize::load_history
    pub async fn get_history(
        &self,
        wrapping_key: &EcEncryptionKey,
    ) -> Result<PrivateNodeOnPathHistory> {
        let mut fs_metadata = FsMetadata::unlock(wrapping_key, &self.metadata).await?;
        Ok(fs_metadata.history(&self.metadata).await?)
    }

    /// Get the Metadata struct which can be used to create Metadata API requests
    pub async fn get_metadata(&self) -> Result<Metadata> {
        let remote_id = self
            .remote_id
            .ok_or(anyhow::anyhow!("remote id not found"))?;
        let root_cid = self
            .content
            .get_root()
            .ok_or(anyhow::anyhow!("root_cid not found"))?;

        let metadata_cid = self
            .metadata
            .get_root()
            .ok_or(anyhow::anyhow!("metadata_cid not found"))?;

        Ok(Metadata {
            id: Uuid::new_v4(),
            bucket_id: remote_id,
            root_cid: root_cid.to_string(),
            metadata_cid: metadata_cid.to_string(),
            data_size: compute_directory_size(&self.content.path)? as u64,
            state: MetadataState::Current,
        })
    }
}

#[cfg(test)]
mod test {
    use std::{
        fs::{create_dir_all, remove_dir_all},
        path::Path,
    };

    use crate::types::config::globalconfig::GlobalConfig;
    use anyhow::Result;
    use chrono::Utc;
    use rand::thread_rng;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn get_set_get_all() -> Result<()> {
        let test_name = "config_set_get_all";
        let origin = &Path::new("test").join(test_name);
        if origin.exists() {
            remove_dir_all(origin)?;
        }
        create_dir_all(origin)?;

        let mut global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.clone().wrapping_key().await?;
        let mut config = global.get_or_create_bucket(origin).await?;

        let rng = &mut thread_rng();
        let fs = &mut config.unlock_fs(&global.wrapping_key().await?).await?;
        config.content.add_delta()?;
        let file = fs
            .root_dir
            .open_file_mut(
                &["cat.png".to_string()],
                true,
                Utc::now(),
                &mut fs.metadata_forest,
                &config.metadata,
                rng,
            )
            .await?;
        let file_content = "this is a cat image".as_bytes();
        file.set_content(
            Utc::now(),
            file_content,
            &mut fs.content_forest,
            &config.content,
            rng,
        )
        .await?;

        config.save_fs(fs).await?;

        // Get structs
        let new_fs = &mut config.unlock_fs(&wrapping_key).await?;

        assert_eq!(fs.root_dir, new_fs.root_dir);

        let new_file = new_fs
            .root_dir
            .open_file_mut(
                &["cat.png".to_string()],
                true,
                Utc::now(),
                &mut new_fs.metadata_forest,
                &config.metadata,
                rng,
            )
            .await?;
        let new_file_content = new_file
            .get_content(&new_fs.content_forest, &config.content)
            .await?;

        assert_eq!(file_content, new_file_content);

        Ok(())
    }
}
