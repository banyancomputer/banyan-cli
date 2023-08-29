use anyhow::{Ok, Result};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
    rc::Rc,
};
use tomb_common::{
    blockstore::{carv2_disk::CarV2DiskBlockStore, multi_carv2_disk::MultiCarV2DiskBlockStore},
    metadata::FsMetadata,
    share::manager::ShareManager,
};
use tomb_crypt::prelude::*;
use uuid::Uuid;
use wnfs::private::{PrivateDirectory, PrivateForest, PrivateNodeOnPathHistory};

use crate::utils::config::xdg_data_home;

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
    /// Randomly generated folder name which holds packed content and key files
    pub(crate) local_id: String,
    /// BlockStore for storing metadata only
    pub metadata: CarV2DiskBlockStore,
    /// BlockStore for storing metadata and file content
    pub content: MultiCarV2DiskBlockStore,
    /// Bucket Uuid, if this
    pub id: Option<Uuid>,
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
        // Save our fs metadata in both of our stores
        fs_metadata.save(&metadata, &metadata).await?;

        Ok(Self {
            name,
            origin: origin.to_path_buf(),
            local_id,
            metadata,
            content,
            id: None,
        })
    }

    pub(crate) fn remove_data(&self) -> Result<()> {
        // Remove dir if it exists
        if bucket_data_home(&self.local_id).exists() {
            remove_dir_all(bucket_data_home(&self.local_id))?;
        }
        Ok(())
    }

    /// Shortcut for serialize::load_all
    pub async fn get_all(
        &self,
        wrapping_key: &EcEncryptionKey,
    ) -> Result<(
        Rc<PrivateForest>,
        Rc<PrivateForest>,
        Rc<PrivateDirectory>,
        ShareManager,
    )> {
        let fs_metadata = FsMetadata::unlock(wrapping_key, &self.metadata).await?;
        Ok((
            fs_metadata.metadata_forest,
            fs_metadata.content_forest,
            fs_metadata.root_dir,
            fs_metadata.share_manager,
        ))
    }

    /// Shortcut for serialize::store_all
    pub async fn set_all(
        &self,
        metadata_forest: &mut Rc<PrivateForest>,
        content_forest: &mut Rc<PrivateForest>,
        root_dir: &Rc<PrivateDirectory>,
        share_manager: &mut ShareManager,
    ) -> Result<()> {
        let mut fs_metadata = FsMetadata {
            metadata_forest: metadata_forest.clone(),
            content_forest: content_forest.clone(),
            root_dir: root_dir.clone(),
            share_manager: share_manager.clone(),
        };
        fs_metadata.save(&self.metadata, &self.content).await?;
        Ok(())
    }

    /// Shortcut for serialize::load_history
    pub async fn get_history(
        &self,
        wrapping_key: &EcEncryptionKey,
    ) -> Result<PrivateNodeOnPathHistory> {
        let mut fs_metadata = FsMetadata::unlock(wrapping_key, &self.metadata).await?;
        Ok(fs_metadata.history(&self.metadata).await?)
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
        let (mut metadata_forest, mut content_forest, mut root_dir, mut share_manager) =
            config.get_all(&global.wrapping_key().await?).await?;
        config.content.add_delta()?;
        let file = root_dir
            .open_file_mut(
                &["cat.png".to_string()],
                true,
                Utc::now(),
                &mut metadata_forest,
                &config.metadata,
                rng,
            )
            .await?;
        let file_content = "this is a cat image".as_bytes();
        file.set_content(
            Utc::now(),
            file_content,
            &mut content_forest,
            &config.content,
            rng,
        )
        .await?;

        config
            .set_all(
                &mut metadata_forest,
                &mut content_forest,
                &root_dir,
                &mut share_manager,
            )
            .await?;

        // Get structs
        let (_new_metadata_forest, _new_content_forest, new_root_dir, _new_manager) =
            config.get_all(&wrapping_key).await?;

        assert_eq!(root_dir, new_root_dir);

        let new_file = root_dir
            .open_file_mut(
                &["cat.png".to_string()],
                true,
                Utc::now(),
                &mut metadata_forest,
                &config.metadata,
                rng,
            )
            .await?;
        let new_file_content = new_file
            .get_content(&content_forest, &config.content)
            .await?;

        assert_eq!(file_content, new_file_content);

        Ok(())
    }
}
