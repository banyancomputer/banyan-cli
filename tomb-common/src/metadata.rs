use anyhow::Result;
use chrono::Utc;
use rand::thread_rng;
use std::{collections::BTreeMap, rc::Rc};
use tomb_crypt::prelude::*;
use wnfs::{
    common::BlockStore,
    libipld::{Cid, Ipld},
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateForest, PrivateNodeOnPathHistory},
};

use crate::{
    blockstore::RootedBlockStore, share::manager::ShareManager, utils::error::SerialError,
    utils::serialize::*,
};

const SHARE_MANAGER_LABEL: &str = "SHARE_MANAGER";
const METADATA_FOREST_LABEL: &str = "METADATA_FOREST";
const CONTENT_FOREST_LABEL: &str = "CONTENT_FOREST";
const TOMB_BUILD_FEATURES_LABEL: &str = "TOMB_BUILD_FEATURES";
const TOMB_BUILD_PROFILE_LABEL: &str = "TOMB_BUILD_PROFILE";
const TOMB_REPO_VERSION_LABEL: &str = "TOMB_REPO_VERSION";

// TODO: Allow ser / de against a cbor file on disk -- that would be straight up easier to debug
/// Describes how to serialize / deserialize metadata for a Wnfs Fs against
/// * a tomb blockstore
#[derive(Debug)]
pub struct FsMetadata {
    /// Private Forest over File systems Metadata blocks
    pub metadata_forest: Rc<PrivateForest>,
    /// Private Forest over File system Content blocks
    pub content_forest: Rc<PrivateForest>,
    /// Reference to the root directory of the Fs
    pub root_dir: Rc<PrivateDirectory>,
    /// Serialized key share
    pub share_manager: ShareManager,
}

impl FsMetadata {
    /// Initialize a new FsMetadata with a wrapping key in memory
    pub async fn init(wrapping_key: &EcEncryptionKey) -> Result<Self> {
        // Create a new PrivateForest for our metadata blocks
        let metadata_forest = Rc::new(PrivateForest::new());
        // Create a new PrivateForest for our content holding blocks
        let content_forest = Rc::new(PrivateForest::new());
        // Create a new PrivateDirectory for the root of the Fs
        let root_dir = Rc::new(PrivateDirectory::new(
            Namefilter::default(),
            Utc::now(),
            &mut thread_rng(),
        ));
        // Create a new Share Manager to hold key shares
        let mut share_manager = ShareManager::default();
        // Insert the initial wrapping key into the key manager
        // Note: this `expect` is needed to get this compiling to wasm
        share_manager
            .share_with(&wrapping_key.public_key().expect("public key not available"))
            .await?;
        // Return the new metadata
        Ok(Self {
            metadata_forest,
            content_forest,
            root_dir,
            share_manager,
        })
    }

    /// Save our metadata as blocks and Link them to the root of the blockstore
    pub async fn save(
        &mut self,
        metadata_store: &impl RootedBlockStore,
        content_store: &impl RootedBlockStore,
    ) -> Result<()> {
        // Store the root directory, get a new PrivateReference to the entry point of the Filesystem
        let root_dir_ref = store_dir(
            metadata_store,
            content_store,
            &mut self.metadata_forest,
            &mut self.content_forest,
            &self.root_dir,
        )
        .await?;
        // Update the private ref in the share manager
        self.share_manager.set_current_ref(&root_dir_ref).await?;

        // Try getting the root, if none is set then its safe to set the original ref as well
        match metadata_store.get_root() {
            None => self.share_manager.set_original_ref(&root_dir_ref).await?,
            Some(cid) => {
                if cid == Cid::default() {
                    self.share_manager.set_original_ref(&root_dir_ref).await?
                }
            }
        }

        // TODO: Can we get away with merging these somehow?
        // Put the forests in the store
        let metadata_forest_cid =
            store_forest(&self.metadata_forest, metadata_store, metadata_store).await?;
        let _metadata_forest_cid =
            store_forest(&self.metadata_forest, metadata_store, content_store).await?;
        let content_forest_cid =
            store_forest(&self.content_forest, content_store, metadata_store).await?;
        let _content_forest_cid =
            store_forest(&self.content_forest, content_store, content_store).await?;
        assert_eq!(metadata_forest_cid, _metadata_forest_cid);
        assert_eq!(content_forest_cid, _content_forest_cid);

        // Put the share manager in the store
        let share_manager_cid = store_share_manager(&self.share_manager, metadata_store).await?;
        let _share_manager_cid = store_share_manager(&self.share_manager, content_store).await?;
        assert_eq!(share_manager_cid, _share_manager_cid);

        // Now for some linking magic
        // Construct new map for metadata
        let mut map = BTreeMap::new();

        // Link everything in the map
        // Link our share manager
        map.insert(
            SHARE_MANAGER_LABEL.to_string(),
            Ipld::Link(share_manager_cid),
        );
        // Link our Private Forests
        map.insert(
            METADATA_FOREST_LABEL.to_string(),
            Ipld::Link(metadata_forest_cid),
        );
        map.insert(
            CONTENT_FOREST_LABEL.to_string(),
            Ipld::Link(content_forest_cid),
        );
        // Link our build metadata
        map.insert(
            TOMB_BUILD_FEATURES_LABEL.to_string(),
            Ipld::String(env!("BUILD_FEATURES").to_string()),
        );
        map.insert(
            TOMB_BUILD_PROFILE_LABEL.to_string(),
            Ipld::String(env!("BUILD_PROFILE").to_string()),
        );
        map.insert(
            TOMB_REPO_VERSION_LABEL.to_string(),
            Ipld::String(env!("REPO_VERSION").to_string()),
        );

        // Get the CID of the map
        let metadata_root = &Ipld::Map(map);
        // Put the metadata IPLD Map into BlockStores
        let metadata_root_cid = metadata_store.put_serializable(metadata_root).await?;
        let _metadata_root_cid = content_store.put_serializable(metadata_root).await?;
        assert_eq!(metadata_root_cid, _metadata_root_cid);

        metadata_store.set_root(&metadata_root_cid);
        content_store.set_root(&metadata_root_cid);

        let _metadata_root_cid = metadata_store
            .get_root()
            .ok_or(SerialError::MissingMetadata("root".to_string()))?;
        assert_eq!(metadata_root_cid, _metadata_root_cid);
        let _metadata_root_cid = content_store
            .get_root()
            .ok_or(SerialError::MissingMetadata("root".to_string()))?;
        assert_eq!(metadata_root_cid, _metadata_root_cid);

        Ok(())
    }

    /// Unlock and initialize Metadata from a blockstore
    pub async fn unlock(
        wrapping_key: &EcEncryptionKey,
        store: &impl RootedBlockStore,
    ) -> Result<Self> {
        // Get the map
        let metadata_root_cid = store
            .get_root()
            .ok_or(SerialError::MissingMetadata("root".to_string()))?;
        let metadata_root = match store.get_deserializable::<Ipld>(&metadata_root_cid).await {
            Ok(Ipld::Map(map)) => map,
            _ => return Err(SerialError::MissingMetadata("IPLD Map".to_string()).into()),
        };
        // Get the forest CIDs
        let metadata_forest_cid = match metadata_root.get(METADATA_FOREST_LABEL) {
            Some(Ipld::Link(cid)) => cid,
            _ => {
                return Err(SerialError::MissingMetadata(METADATA_FOREST_LABEL.to_string()).into())
            }
        };
        let content_forest_cid = match metadata_root.get(CONTENT_FOREST_LABEL) {
            Some(Ipld::Link(cid)) => cid,
            _ => return Err(SerialError::MissingMetadata(CONTENT_FOREST_LABEL.to_string()).into()),
        };
        // Get the share manager CID
        let share_manager_cid = match metadata_root.get(SHARE_MANAGER_LABEL) {
            Some(Ipld::Link(cid)) => cid,
            _ => return Err(SerialError::MissingMetadata(SHARE_MANAGER_LABEL.to_string()).into()),
        };

        // Get the forests
        let metadata_forest = load_forest(metadata_forest_cid, store).await?;
        let content_forest = load_forest(content_forest_cid, store).await?;
        // Get the share manager
        let mut share_manager = store
            .get_deserializable::<ShareManager>(share_manager_cid)
            .await?;
        // Get our private Ref
        share_manager.load_refs(wrapping_key).await?;
        let current_private_ref =
            share_manager
                .current_ref
                .as_ref()
                .ok_or(SerialError::MissingMetadata(
                    "current private ref".to_string(),
                ))?;

        // Get the root directory
        let root_dir = load_dir(store, current_private_ref, &metadata_forest).await?;
        // Return the new metadata
        Ok(Self {
            metadata_forest,
            content_forest,
            root_dir,
            share_manager,
        })
    }

    /// Get the original root directory
    pub async fn history(&mut self, store: &impl BlockStore) -> Result<PrivateNodeOnPathHistory> {
        // Get the original private ref
        let original_private_ref =
            self.share_manager
                .original_ref
                .as_ref()
                .ok_or(SerialError::MissingMetadata(
                    "original private ref".to_string(),
                ))?;
        // Get the original root directory
        let original_root_dir =
            load_dir(store, original_private_ref, &self.metadata_forest).await?;
        // Get the history
        PrivateNodeOnPathHistory::of(
            self.root_dir.clone(),
            original_root_dir,
            1_000_000,
            &[],
            true,
            self.metadata_forest.clone(),
            store,
        )
        .await
    }

    /// Return the build details
    pub async fn build_details(
        &self,
        store: &impl RootedBlockStore,
    ) -> Result<(String, String, String)> {
        // Get the map
        let metadata_root_cid = store
            .get_root()
            .ok_or(SerialError::MissingMetadata("root".to_string()))?;
        let metadata_root = match store.get_deserializable::<Ipld>(&metadata_root_cid).await {
            Ok(Ipld::Map(map)) => map,
            _ => return Err(SerialError::MissingMetadata("IPLD Map".to_string()).into()),
        };
        // Get the build details
        let build_features = match metadata_root.get(TOMB_BUILD_FEATURES_LABEL) {
            Some(Ipld::String(build_features)) => build_features,
            _ => {
                return Err(
                    SerialError::MissingMetadata(TOMB_BUILD_FEATURES_LABEL.to_string()).into(),
                )
            }
        };
        let build_profile = match metadata_root.get(TOMB_BUILD_PROFILE_LABEL) {
            Some(Ipld::String(build_profile)) => build_profile,
            _ => {
                return Err(
                    SerialError::MissingMetadata(TOMB_BUILD_PROFILE_LABEL.to_string()).into(),
                )
            }
        };
        let repo_version = match metadata_root.get(TOMB_REPO_VERSION_LABEL) {
            Some(Ipld::String(repo_version)) => repo_version,
            _ => {
                return Err(
                    SerialError::MissingMetadata(TOMB_REPO_VERSION_LABEL.to_string()).into(),
                )
            }
        };
        // Ok
        Ok((
            build_features.to_string(),
            build_profile.to_string(),
            repo_version.to_string(),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::blockstore::memory::MemoryBlockStore;
    use anyhow::Result;
    use serial_test::serial;

    async fn _init_save_unlock(
        wrapping_key: &EcEncryptionKey,
        metadata_store: &mut MemoryBlockStore,
        content_store: &mut MemoryBlockStore,
    ) -> Result<FsMetadata> {
        let mut metadata = FsMetadata::init(wrapping_key).await?;
        metadata.save(metadata_store, content_store).await?;
        let unlocked_metadata = FsMetadata::unlock(wrapping_key, metadata_store).await?;
        assert_eq!(metadata.root_dir, unlocked_metadata.root_dir);
        assert_eq!(metadata.share_manager, unlocked_metadata.share_manager);
        Ok(unlocked_metadata)
    }

    #[tokio::test]
    #[serial]
    async fn init_save_unlock() -> Result<()> {
        let metadata_store = &mut MemoryBlockStore::default();
        let content_store = &mut MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let _ = _init_save_unlock(wrapping_key, metadata_store, content_store).await?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn history() -> Result<()> {
        let metadata_store = &mut MemoryBlockStore::default();
        let content_store = &mut MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let mut metadata = _init_save_unlock(wrapping_key, metadata_store, content_store).await?;
        let _history = metadata.history(metadata_store).await?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn build_details() -> Result<()> {
        let metadata_store = &mut MemoryBlockStore::default();
        let content_store = &mut MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let metadata = _init_save_unlock(wrapping_key, metadata_store, content_store).await?;
        let _build_details = metadata.build_details(metadata_store).await?;
        Ok(())
    }
}
