use crate::{
    blockstore::RootedBlockStore,
    share::manager::ShareManager,
    utils::error::SerialError,
    utils::{serialize::*, wnfsio::*},
};
use anyhow::Result;
use chrono::Utc;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, rc::Rc};
use tomb_crypt::prelude::*;
use wnfs::{
    common::{BlockStore, Metadata},
    libipld::{Cid, Ipld},
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateForest, PrivateNode, PrivateNodeOnPathHistory},
};

const SHARE_MANAGER_LABEL: &str = "SHARE_MANAGER";
const METADATA_FOREST_LABEL: &str = "METADATA_FOREST";
const CONTENT_FOREST_LABEL: &str = "CONTENT_FOREST";
const ROOT_MAP_LABEL: &str = "ROOT_MAP";
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
    /// Loaded Metadata
    pub metadata: Option<BTreeMap<String, Ipld>>,
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
            metadata: None,
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
        // Construct a new map for the forests
        let mut root_map = BTreeMap::new();
        // Link our Private Forests
        root_map.insert(
            METADATA_FOREST_LABEL.to_string(),
            Ipld::Link(metadata_forest_cid),
        );
        root_map.insert(
            CONTENT_FOREST_LABEL.to_string(),
            Ipld::Link(content_forest_cid),
        );
        // Put the map into BlockStores
        let root = &Ipld::Map(root_map);
        let root_cid = metadata_store.put_serializable(root).await?;
        let _root_cid = content_store.put_serializable(root).await?;
        assert_eq!(root_cid, _root_cid);

        // Construct new map for metadata
        let mut metadata_map = BTreeMap::new();
        // Link our forests
        metadata_map.insert(ROOT_MAP_LABEL.to_string(), Ipld::Link(root_cid));
        // Link our share manager
        metadata_map.insert(
            SHARE_MANAGER_LABEL.to_string(),
            Ipld::Link(share_manager_cid),
        );
        // Link our build metadata
        metadata_map.insert(
            TOMB_BUILD_FEATURES_LABEL.to_string(),
            Ipld::String(env!("BUILD_FEATURES").to_string()),
        );
        metadata_map.insert(
            TOMB_BUILD_PROFILE_LABEL.to_string(),
            Ipld::String(env!("BUILD_PROFILE").to_string()),
        );
        metadata_map.insert(
            TOMB_REPO_VERSION_LABEL.to_string(),
            Ipld::String(env!("REPO_VERSION").to_string()),
        );

        // Get the CID of the metadata map
        let metadata = &Ipld::Map(metadata_map.clone());
        // Put the metadata IPLD Map into BlockStores
        let metadata_cid = metadata_store.put_serializable(metadata).await?;
        let _metadata_cid = content_store.put_serializable(metadata).await?;
        assert_eq!(metadata_cid, _metadata_cid);

        metadata_store.set_root(&metadata_cid);
        content_store.set_root(&metadata_cid);

        let _metadata_cid = metadata_store
            .get_root()
            .ok_or(SerialError::MissingMetadata("root cid".to_string()))?;
        assert_eq!(metadata_cid, _metadata_cid);
        let _metadata_cid = content_store
            .get_root()
            .ok_or(SerialError::MissingMetadata("root cid".to_string()))?;
        assert_eq!(metadata_cid, _metadata_cid);

        self.metadata = Some(metadata_map);
        Ok(())
    }

    /// Unlock and initialize Metadata from a blockstore
    pub async fn unlock(
        wrapping_key: &EcEncryptionKey,
        store: &impl RootedBlockStore,
    ) -> Result<Self> {
        // Get the map
        let metadata_cid = store
            .get_root()
            .ok_or(SerialError::MissingMetadata("root cid".to_string()))?;
        let metadata = match store.get_deserializable::<Ipld>(&metadata_cid).await {
            Ok(Ipld::Map(map)) => map,
            _ => return Err(SerialError::MissingMetadata("metadata map".to_string()).into()),
        };
        // Get the CID of the forest Map
        let root_cid = match metadata.get(ROOT_MAP_LABEL) {
            Some(Ipld::Link(cid)) => cid,
            _ => return Err(SerialError::MissingMetadata(ROOT_MAP_LABEL.to_string()).into()),
        };
        // Get the forest Map
        let root = match store.get_deserializable::<Ipld>(root_cid).await {
            Ok(Ipld::Map(map)) => map,
            _ => return Err(SerialError::MissingMetadata("root map".to_string()).into()),
        };
        // Get the metadata forest CID
        let metadata_forest_cid = match root.get(METADATA_FOREST_LABEL) {
            Some(Ipld::Link(cid)) => cid,
            _ => {
                return Err(SerialError::MissingMetadata(METADATA_FOREST_LABEL.to_string()).into())
            }
        };
        // Get the content forest CID
        let content_forest_cid = match root.get(CONTENT_FOREST_LABEL) {
            Some(Ipld::Link(cid)) => cid,
            _ => return Err(SerialError::MissingMetadata(CONTENT_FOREST_LABEL.to_string()).into()),
        };
        // Get the share manager CID
        let share_manager_cid = match metadata.get(SHARE_MANAGER_LABEL) {
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
            metadata: Some(metadata),
        })
    }

    /// Share with
    pub async fn share_with(
        &mut self,
        recipient: &EcPublicEncryptionKey,
        store: &impl RootedBlockStore,
    ) -> Result<()> {
        self.share_manager.share_with(recipient).await?;
        // Save the new share manager to the map, conserving all other links in the metadata
        let store_manager_cid = store_share_manager(&self.share_manager, store).await?;
        // Get the map
        let mut metadata = self
            .metadata
            .as_ref()
            .ok_or(SerialError::MissingMetadata("metadata map".to_string()))?
            .clone();
        // Update the share manager link
        metadata.insert(
            SHARE_MANAGER_LABEL.to_string(),
            Ipld::Link(store_manager_cid),
        );

        self.metadata = Some(metadata.clone());
        // Get the CID of the metadata map
        let metadata = &Ipld::Map(metadata);
        // Put the metadata IPLD Map into BlockStores
        let metadata_cid = store.put_serializable(metadata).await?;
        // Update the root CID
        store.set_root(&metadata_cid);
        // Update the metadata
        Ok(())
    }

    /// Get the metadata cid from the blockstore
    /// This should just be the root cid
    pub async fn metadata_cid(&self, store: &impl RootedBlockStore) -> Result<Cid> {
        store
            .get_root()
            .ok_or(SerialError::MissingMetadata("root cid".to_string()).into())
    }

    /// Get the root cid from the blockstore
    pub async fn root_cid(&self, store: &impl RootedBlockStore) -> Result<Cid> {
        // Get the map
        let metadata_cid = store
            .get_root()
            .ok_or(SerialError::MissingMetadata("root cid".to_string()))?;
        let metadata = match store.get_deserializable::<Ipld>(&metadata_cid).await {
            Ok(Ipld::Map(map)) => map,
            _ => return Err(SerialError::MissingMetadata("metadata map".to_string()).into()),
        };
        // Get the CID of the forest Map
        let root_cid = match metadata.get(ROOT_MAP_LABEL) {
            Some(Ipld::Link(cid)) => cid,
            _ => return Err(SerialError::MissingMetadata(ROOT_MAP_LABEL.to_string()).into()),
        };
        Ok(*root_cid)
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
        let metadata_cid = store
            .get_root()
            .ok_or(SerialError::MissingMetadata("root cid".to_string()))?;
        let metadata = match store.get_deserializable::<Ipld>(&metadata_cid).await {
            Ok(Ipld::Map(map)) => map,
            _ => return Err(SerialError::MissingMetadata("metadata map".to_string()).into()),
        };
        // Get the build details
        let build_features = match metadata.get(TOMB_BUILD_FEATURES_LABEL) {
            Some(Ipld::String(build_features)) => build_features,
            _ => {
                return Err(
                    SerialError::MissingMetadata(TOMB_BUILD_FEATURES_LABEL.to_string()).into(),
                )
            }
        };
        let build_profile = match metadata.get(TOMB_BUILD_PROFILE_LABEL) {
            Some(Ipld::String(build_profile)) => build_profile,
            _ => {
                return Err(
                    SerialError::MissingMetadata(TOMB_BUILD_PROFILE_LABEL.to_string()).into(),
                )
            }
        };
        let repo_version = match metadata.get(TOMB_REPO_VERSION_LABEL) {
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

    /// Make a new directory in the Fs. Store in our metadata store
    pub async fn mkdir(
        &mut self,
        path_segments: Vec<String>,
        store: &impl RootedBlockStore,
    ) -> Result<()> {
        // Search through the PrivateDirectory for a Node that matches the path provided
        let result = self
            .root_dir
            .get_node(&path_segments, true, &self.metadata_forest, store)
            .await;

        if let Ok(node) = result && node.is_some() {}
        // If there was an error searching for the Node or
        else {
            // Create the subdirectory
            self.root_dir
                .mkdir(
                    &path_segments,
                    true,
                    Utc::now(),
                    &self.metadata_forest,
                    store,
                    &mut thread_rng(),
                )
                .await?;
        }
        Ok(())
    }

    /// Ls the root directory at the path provided
    pub async fn ls(
        &self,
        path_segments: Vec<String>,
        store: &impl RootedBlockStore,
    ) -> Result<Vec<FsMetadataEntry>> {
        let fetched_entries = self
            .root_dir
            .ls(&path_segments, true, &self.metadata_forest, store)
            .await?;

        let mut transformed_entries = Vec::with_capacity(fetched_entries.len());
        let mut futures = Vec::new();

        for (name, metadata) in fetched_entries.iter() {
            let node_path_segments = path_segments
                .iter()
                .chain(std::iter::once(name))
                .cloned()
                .collect::<Vec<String>>();

            let future = async move {
                // Get the node from the path
                let entry = self
                    .root_dir
                    .get_node(&node_path_segments, true, &self.metadata_forest, store)
                    .await
                    .expect("node not found");
                let entry = entry
                    .ok_or(SerialError::NodeNotFound(
                        node_path_segments.join("/").to_string(),
                    ))
                    .expect("node not found");
                // Map the node to an FsMetadataEntry
                let name = name.to_string();
                let entry_type = match entry {
                    PrivateNode::Dir(_) => FsMetadataEntryType::Dir,
                    PrivateNode::File(_) => FsMetadataEntryType::File,
                };
                let metadata = metadata.clone();
                FsMetadataEntry {
                    name,
                    entry_type,
                    metadata,
                }
            };

            futures.push(future);
        }

        // Since the transformation is async, await all futures
        for future in futures {
            let entry = future.await;
            transformed_entries.push(entry);
        }

        Ok(transformed_entries)
    }

    /// Add a Vector of bytes as a new file in the Fs. Store in our content store
    pub async fn add(
        &mut self,
        path_segments: Vec<String>,
        content: Vec<u8>,
        metadata_store: &impl RootedBlockStore,
        content_store: &impl RootedBlockStore,
    ) -> Result<()> {
        // Compress the data in the file
        let content_buf = compress_vec(&content)?;
        // Turn the relative path into a vector of segments
        let time = Utc::now();
        let rng = &mut thread_rng();
        let file = self
            .root_dir
            .open_file_mut(
                &path_segments,
                true,
                time,
                &mut self.metadata_forest,
                metadata_store,
                rng,
            )
            .await?;

        // Set file contents
        file.set_content(
            time,
            content_buf.as_slice(),
            &mut self.content_forest,
            content_store,
            rng,
        )
        .await?;

        // Ok
        Ok(())
    }

    /// Add a Vector of bytes as a new file in the Fs. Store in our content store
    pub async fn read(
        &mut self,
        path_segments: Vec<String>,
        metadata_store: &impl RootedBlockStore,
        content_store: &impl BlockStore,
    ) -> Result<Vec<u8>> {
        // Compress the data in the file
        let result = self
            .root_dir
            .get_node(&path_segments, true, &self.metadata_forest, metadata_store)
            .await
            .expect("node not found");
        match result {
            Some(PrivateNode::File(file)) => {
                let content = file
                    .get_content(&self.content_forest, content_store)
                    .await?;
                let content = decompress_vec(&content)?;
                Ok(content)
            }
            _ => Err(SerialError::NodeNotFound(path_segments.join("/").to_string()).into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Dirty enum describing the type of a FsMetadataEntry
pub enum FsMetadataEntryType {
    /// Dir
    Dir,
    /// File
    File,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Helper struct to return FsMetadataEntry
pub struct FsMetadataEntry {
    /// File / Dir name
    pub name: String,
    /// File / Dir type
    pub entry_type: FsMetadataEntryType,
    /// File / Dir metadata
    pub metadata: Metadata,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::blockstore::memory::MemoryBlockStore;
    use anyhow::Result;
    use serial_test::serial;

    async fn _init_save_unlock(
        wrapping_key: &EcEncryptionKey,
        metadata_store: &MemoryBlockStore,
        content_store: &MemoryBlockStore,
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