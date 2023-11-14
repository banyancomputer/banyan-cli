use crate::{
    api::models::storage_ticket::StorageTicket,
    blockstore::{CarV2DiskBlockStore, MultiCarV2DiskBlockStore},
    filesystem::{FilesystemError, FsMetadata},
    native::configuration::{xdg::xdg_data_home, ConfigurationError},
};
use colored::Colorize;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fmt::Display,
    fs::{create_dir_all, remove_dir_all},
    path::{Path, PathBuf},
};
use tomb_crypt::prelude::{EcEncryptionKey, PrivateKey};
use uuid::Uuid;
use wnfs::{libipld::Cid, private::PrivateNodeOnPathHistory};

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
pub struct LocalBucket {
    /// The name of this bucket
    pub(crate) name: String,
    /// The filesystem that this bucket represents
    pub(crate) origin: PathBuf,
    /// Randomly generated folder name which holds prepared content and key files
    local_id: String,
    /// Bucket Uuid on the remote server
    pub(crate) remote_id: Option<Uuid>,
    /// Storage ticket in case we lose track of non-metadata components
    pub(crate) storage_ticket: Option<StorageTicket>,
    /// Locally deleted blocks the server needs to be notified of
    pub(crate) deleted_block_cids: BTreeSet<Cid>,
    /// BlockStore for storing metadata only
    pub metadata: CarV2DiskBlockStore,
    /// BlockStore for storing metadata and file content
    pub content: MultiCarV2DiskBlockStore,
}

impl Display for LocalBucket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "name:\t\t\t{}\ndrive_id:\t\t{}\norigin:\t\t\t{}\nstorage_ticket:\t\t{}",
            self.name,
            if let Some(remote_id) = self.remote_id {
                remote_id.to_string()
            } else {
                format!("{}", "Unknown".red())
            },
            self.origin.display(),
            if let Some(storage_ticket) = self.storage_ticket.clone() {
                storage_ticket.host
            } else {
                format!("{}", "None".yellow())
            }
        ))
    }
}

impl LocalBucket {
    /// Given a directory, initialize a configuration for it
    pub async fn new(
        origin: &Path,
        wrapping_key: &EcEncryptionKey,
    ) -> Result<Self, ConfigurationError> {
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
        let mut content = MultiCarV2DiskBlockStore::new(&content_path)?;
        content.add_delta()?;

        // Initialize the fs metadata
        let mut fs_metadata = FsMetadata::init(wrapping_key).await?;
        let public_key = wrapping_key.public_key()?;

        // Save our fs to establish map
        fs_metadata.save(&metadata, &content).await?;
        // Share it with the owner of this wrapping key
        fs_metadata.share_with(&public_key, &metadata).await?;
        // Save our fs again
        fs_metadata.save(&metadata, &content).await?;

        Ok(Self {
            name,
            origin: origin.to_path_buf(),
            local_id,
            remote_id: None,
            storage_ticket: None,
            deleted_block_cids: BTreeSet::new(),
            metadata,
            content,
        })
    }

    pub(crate) fn remove_data(&self) -> Result<(), std::io::Error> {
        // Remove dir if it exists
        if bucket_data_home(&self.local_id).exists() {
            remove_dir_all(bucket_data_home(&self.local_id))?;
        }
        Ok(())
    }

    /// Shortcut for unlocking a filesystem
    pub async fn unlock_fs(
        &self,
        wrapping_key: &EcEncryptionKey,
    ) -> Result<FsMetadata, FilesystemError> {
        FsMetadata::unlock(wrapping_key, &self.metadata).await
    }

    /// Shortcut for saving a filesystem
    pub async fn save_fs(&self, fs: &mut FsMetadata) -> Result<(), FilesystemError> {
        fs.save(&self.metadata, &self.content).await
    }

    /// Shortcut for serialize::load_history
    pub async fn get_history(
        &self,
        wrapping_key: &EcEncryptionKey,
    ) -> Result<PrivateNodeOnPathHistory, FilesystemError> {
        let mut fs_metadata = FsMetadata::unlock(wrapping_key, &self.metadata).await?;
        Ok(fs_metadata.history(&self.metadata).await?)
    }
}

#[cfg(test)]
mod test {
    use crate::native::configuration::{globalconfig::GlobalConfig, ConfigurationError};
    use chrono::Utc;
    use rand::thread_rng;
    use serial_test::serial;
    use std::{
        fs::{create_dir_all, remove_dir_all},
        path::Path,
    };

    #[tokio::test]
    #[serial]
    async fn get_set_get_all() -> Result<(), ConfigurationError> {
        let test_name = "config_set_get_all";
        let origin = Path::new("test").join(test_name);
        if origin.exists() {
            remove_dir_all(&origin)?;
        }
        create_dir_all(&origin)?;

        let mut global = GlobalConfig::from_disk().await?;
        let wrapping_key = global.clone().wrapping_key().await?;
        let mut config = global.get_or_init_bucket("test", &origin).await?;

        let mut rng = thread_rng();
        let mut fs = config.unlock_fs(&global.wrapping_key().await?).await?;
        config.content.add_delta()?;
        let file = fs
            .root_dir
            .open_file_mut(
                &["cat.png".to_string()],
                true,
                Utc::now(),
                &mut fs.forest,
                &config.metadata,
                &mut rng,
            )
            .await?;
        let file_content = "this is a cat image".as_bytes();
        file.set_content(
            Utc::now(),
            file_content,
            &mut fs.forest,
            &config.content,
            &mut rng,
        )
        .await?;

        config.save_fs(&mut fs).await?;

        // Get structs
        let mut new_fs = config.unlock_fs(&wrapping_key).await?;

        assert_eq!(fs.root_dir, new_fs.root_dir);

        let new_file = new_fs
            .root_dir
            .open_file_mut(
                &["cat.png".to_string()],
                true,
                Utc::now(),
                &mut new_fs.forest,
                &config.metadata,
                &mut rng,
            )
            .await?;
        let new_file_content = new_file
            .get_content(&new_fs.forest, &config.content)
            .await?;

        assert_eq!(file_content, new_file_content);

        Ok(())
    }
}
