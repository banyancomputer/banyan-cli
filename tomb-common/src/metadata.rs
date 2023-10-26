use crate::{
    blockstore::{carv2_memory::CarV2MemoryBlockStore, split::DoubleSplitStore, RootedBlockStore},
    share::manager::ShareManager,
    utils::serialize::*,
    utils::{error::SerialError, wnfsio::path_to_segments},
};
use anyhow::{anyhow, Result};
use async_recursion::async_recursion;
use chrono::Utc;
use futures_util::future::join_all;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    rc::Rc,
};
use tomb_crypt::prelude::*;
use wnfs::{
    common::{BlockStore, Metadata},
    libipld::{Cid, Ipld},
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateForest, PrivateNode, PrivateNodeOnPathHistory},
};

const SHARE_MANAGER_LABEL: &str = "SHARE_MANAGER";
const FOREST_LABEL: &str = "FOREST";
const TOMB_BUILD_FEATURES_LABEL: &str = "TOMB_BUILD_FEATURES";
const TOMB_BUILD_PROFILE_LABEL: &str = "TOMB_BUILD_PROFILE";
const TOMB_REPO_VERSION_LABEL: &str = "TOMB_REPO_VERSION";

// TODO: Allow ser / de against a cbor file on disk -- that would be straight up easier to debug
/// Describes how to serialize / deserialize metadata for a Wnfs Fs against
/// * a tomb blockstore
#[derive(Debug)]
pub struct FsMetadata {
    /// Private Forest over File system
    pub forest: Rc<PrivateForest>,
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
        // Create a new PrivateForest
        let forest = Rc::new(PrivateForest::new());
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
            forest,
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
            &mut self.forest,
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
        let forest_cid_1 = store_forest(&self.forest, metadata_store, metadata_store).await?;
        let forest_cid_2 = store_forest(&self.forest, metadata_store, content_store).await?;
        assert_eq!(forest_cid_1, forest_cid_2);

        // Put the share manager in the store
        let share_manager_cid_1 = store_share_manager(&self.share_manager, metadata_store).await?;
        let share_manager_cid_2 = store_share_manager(&self.share_manager, content_store).await?;
        assert_eq!(share_manager_cid_1, share_manager_cid_2);

        // Now for some linking magic
        // Construct a new map for the forests
        let mut root_map = BTreeMap::new();
        // Link our Private Forests
        root_map.insert(FOREST_LABEL.to_string(), Ipld::Link(forest_cid_1));

        // Link our forests
        // Link our share manager
        root_map.insert(
            SHARE_MANAGER_LABEL.to_string(),
            Ipld::Link(share_manager_cid_1),
        );
        // Link our build metadata
        root_map.insert(
            TOMB_BUILD_FEATURES_LABEL.to_string(),
            Ipld::String(env!("BUILD_FEATURES").to_string()),
        );
        root_map.insert(
            TOMB_BUILD_PROFILE_LABEL.to_string(),
            Ipld::String(env!("BUILD_PROFILE").to_string()),
        );
        root_map.insert(
            TOMB_REPO_VERSION_LABEL.to_string(),
            Ipld::String(env!("REPO_VERSION").to_string()),
        );

        // Put the map into BlockStores
        let root = Ipld::Map(root_map.clone());
        let root_cid_1 = metadata_store.put_serializable(&root).await?;
        let root_cid_2 = content_store.put_serializable(&root).await?;
        assert_eq!(root_cid_1, root_cid_2);

        metadata_store.set_root(&root_cid_1);
        content_store.set_root(&root_cid_1);

        let root_cid_3 = metadata_store
            .get_root()
            .ok_or(SerialError::MissingMetadata("root cid".to_string()))?;
        assert_eq!(root_cid_1, root_cid_3);
        let root_cid_4 = content_store
            .get_root()
            .ok_or(SerialError::MissingMetadata("root cid".to_string()))?;
        assert_eq!(root_cid_1, root_cid_4);

        self.metadata = Some(root_map);

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
        let root_map = match store.get_deserializable::<Ipld>(&metadata_cid).await {
            Ok(Ipld::Map(map)) => map,
            _ => return Err(SerialError::MissingMetadata("metadata map".to_string()).into()),
        };
        // Get the forest CID
        let forest_cid = match root_map.get(FOREST_LABEL) {
            Some(Ipld::Link(cid)) => cid,
            _ => return Err(SerialError::MissingMetadata(FOREST_LABEL.to_string()).into()),
        };
        // Get the share manager CID
        let share_manager_cid = match root_map.get(SHARE_MANAGER_LABEL) {
            Some(Ipld::Link(cid)) => cid,
            _ => return Err(SerialError::MissingMetadata(SHARE_MANAGER_LABEL.to_string()).into()),
        };

        // Get the forests
        let forest = load_forest(forest_cid, store).await?;
        let forest_store = CarV2MemoryBlockStore::new()?;
        let forest =
            Rc::new(PrivateForest::load(&forest.store(&forest_store).await?, &forest_store).await?);

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
        let root_dir = load_dir(store, current_private_ref, &forest).await?;
        // Return the new metadata
        Ok(Self {
            forest,
            root_dir,
            share_manager,
            metadata: Some(root_map),
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
        let metadata = Ipld::Map(metadata);
        // Put the metadata IPLD Map into BlockStores
        let metadata_cid = store.put_serializable(&metadata).await?;
        // Update the root CID
        store.set_root(&metadata_cid);
        // Update the metadata
        Ok(())
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
        let original_root_dir = load_dir(store, original_private_ref, &self.forest).await?;
        // Get the history
        PrivateNodeOnPathHistory::of(
            self.root_dir.clone(),
            original_root_dir,
            1_000_000,
            &[],
            true,
            self.forest.clone(),
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
        path_segments: &[String],
        metadata_store: &impl RootedBlockStore,
    ) -> Result<()> {
        // Search through the PrivateDirectory for a Node that matches the path provided
        let result = self
            .root_dir
            .get_node(path_segments, true, &self.forest, metadata_store)
            .await;

        if let Ok(node) = result
            && node.is_some()
        {
        }
        // If there was an error searching for the Node or
        else {
            // Create the subdirectory
            self.root_dir
                .mkdir(
                    path_segments,
                    true,
                    Utc::now(),
                    &self.forest,
                    metadata_store,
                    &mut thread_rng(),
                )
                .await?;
        }
        Ok(())
    }

    /// Ls the root directory at the path provided
    pub async fn ls(
        &self,
        path_segments: &[String],
        store: &impl RootedBlockStore,
    ) -> Result<Vec<FsMetadataEntry>> {
        let fetched_entries = self
            .root_dir
            .ls(path_segments, true, &self.forest, store)
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
                    .get_node(&node_path_segments, true, &self.forest, store)
                    .await
                    .expect("node not found");
                let entry = entry
                    .ok_or(SerialError::NodeNotFound(node_path_segments.join("/")))
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

    /// Mv a file or directory to a new location
    pub async fn mv(
        &mut self,
        src_path_segments: &[String],
        dest_path_segments: &[String],
        metadata_store: &impl RootedBlockStore,
        content_store: &impl RootedBlockStore,
    ) -> Result<()> {
        let ds_store = DoubleSplitStore::new(metadata_store, content_store);
        let result = self
            .root_dir
            .get_node(src_path_segments, true, &self.forest, &ds_store)
            .await?;
        match result {
            Some(_) => {
                self.root_dir
                    .basic_mv(
                        src_path_segments,
                        dest_path_segments,
                        true,
                        Utc::now(),
                        &mut self.forest,
                        &ds_store,
                        &mut thread_rng(),
                    )
                    .await
            }
            None => Err(SerialError::NodeNotFound(src_path_segments.join("/").to_string()).into()),
        }
    }

    /// Cp a file to a new location while deduplicating
    pub async fn cp(
        &mut self,
        src_path_segments: &[String],
        dest_path_segments: &[String],
        metadata_store: &impl RootedBlockStore,
    ) -> Result<()> {
        // Get the path of the parent
        let folder_segments = &dest_path_segments[..&dest_path_segments.len() - 1].to_vec();
        // Make directory at parent
        self.mkdir(folder_segments, metadata_store).await?;
        // Copy and Link
        self.root_dir
            .cp_link(
                src_path_segments,
                dest_path_segments,
                true,
                &mut self.forest,
                metadata_store,
            )
            .await
    }

    /// Write a symlink
    pub async fn symlink(
        &mut self,
        target: &Path,
        path_segments: &[String],
        metadata_store: &impl RootedBlockStore,
    ) -> Result<()> {
        // Represent the target as a String
        let target_string = target
            .to_str()
            .expect("failed to represent Path as str")
            .to_string();
        // Write the symlink
        self.root_dir
            .write_symlink(
                target_string,
                path_segments,
                true,
                Utc::now(),
                &self.forest,
                metadata_store,
                &mut thread_rng(),
            )
            .await
    }

    /// Rm a file or directory
    pub async fn rm(
        &mut self,
        path_segments: &[String],
        store: &impl RootedBlockStore,
    ) -> Result<()> {
        // Create the subdirectory
        self.root_dir
            .rm(path_segments, true, &self.forest, store)
            .await
            .map(|_| ())
            .map_err(|_| SerialError::NodeNotFound(path_segments.join("/")).into())
    }

    /// Add a Vector of bytes as a new file in the Fs. Store in our content store
    pub async fn read(
        &mut self,
        path_segments: &[String],
        metadata_store: &impl RootedBlockStore,
        content_store: &impl BlockStore,
    ) -> Result<Vec<u8>> {
        println!("attempting to read: {:?}", path_segments);
        println!("forest: {:?}", self.forest);

        // Compress the data in the file
        let result = self
            .root_dir
            .get_node(path_segments, true, &self.forest, metadata_store)
            .await
            .expect("node not found");

        // Split store for reading
        let split_store = DoubleSplitStore::new(content_store, metadata_store);

        // If the node is found and is a file
        if let Some(PrivateNode::File(file)) = result {
            println!("about to get cids...");
            let all_cids = file.get_cids(&self.forest, metadata_store).await?;
            println!("all_cids: {:?}", all_cids);
            println!("about to get content...");
            let content = file.get_content(&self.forest, &split_store).await?;
            println!("got content...");
            Ok(content)
        } else {
            Err(SerialError::NodeNotFound(path_segments.join("/")).into())
        }
    }

    /// Write data do a specific node
    pub async fn write(
        &mut self,
        path_segments: &[String],
        metadata_store: &impl RootedBlockStore,
        content_store: &impl BlockStore,
        content: Vec<u8>,
    ) -> Result<()> {
        let time = Utc::now();
        let data_size = content.len();
        let mut rng = thread_rng();

        let result = self
            .root_dir
            .open_file_mut(
                path_segments,
                true,
                time,
                &mut self.forest,
                metadata_store,
                &mut rng,
            )
            .await;

        if let Ok(file) = result {
            file.set_content(
                Utc::now(),
                content.as_slice(),
                &mut self.forest,
                content_store,
                &mut thread_rng(),
            )
            .await?;

            let full_path: std::path::PathBuf = path_segments.iter().collect();
            if let Some(mime) = mime_guess::MimeGuess::from_path(full_path).first() {
                file.content
                    .metadata
                    .put("mime_type", Ipld::String(mime.essence_str().to_string()));
            }

            file.content
                .metadata
                .put("size", Ipld::Integer(data_size as i128));

            Ok(())
        } else {
            Err(SerialError::NodeNotFound(path_segments.join("/")).into())
        }
    }

    /// Get a node from the Fs
    pub async fn get_node(
        &mut self,
        path_segments: &[String],
        store: &impl RootedBlockStore,
    ) -> Result<Option<PrivateNode>> {
        // Search through the PrivateDirectory for a Node that matches the path provided
        let result = self
            .root_dir
            .get_node(path_segments, true, &self.forest, store)
            .await;
        match result {
            Ok(node) => Ok(node),
            Err(_) => Err(SerialError::NodeNotFound(path_segments.join("/")).into()),
        }
    }

    /// Get all nodes under the root directory
    pub async fn get_all_nodes(
        &self,
        metadata_store: &impl BlockStore,
    ) -> Result<Vec<(PrivateNode, PathBuf)>> {
        self.get_all_children(Path::new("").to_path_buf(), metadata_store)
            .await
    }

    #[async_recursion(?Send)]
    async fn get_all_children(
        &self,
        path: PathBuf,
        metadata_store: &impl BlockStore,
    ) -> Result<Vec<(PrivateNode, PathBuf)>> {
        let segments = path_to_segments(&path)?;
        let node = if segments.is_empty() {
            Some(self.root_dir.as_node())
        } else {
            self.root_dir
                .get_node(&segments, true, &self.forest, metadata_store)
                .await?
        };

        match node {
            Some(PrivateNode::File(file)) => Ok(vec![(file.as_node(), path.to_path_buf())]),
            Some(PrivateNode::Dir(dir)) => {
                // Accumulate a list
                let mut children = vec![];
                // List the names of all children
                let node_names = dir.ls(&[], true, &self.forest, metadata_store).await?;

                // Accumulate a list of futures
                let mut futures = Vec::new();
                // Add a future for each node name
                for (node_name, _) in node_names {
                    futures.push(self.get_all_children(path.join(node_name), metadata_store));
                }
                // Join on all of them and iterate over results
                for result in join_all(futures).await {
                    // If the result succeeded, extend children found
                    if let Ok(node_children) = result {
                        children.extend(node_children)
                    } else {
                        return Err(anyhow!("unable to recurse"));
                    }
                }
                Ok(children)
            }
            None => Err(anyhow!("invalid node")),
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
    async fn init_save_unlock() -> Result<()> {
        let metadata_store = MemoryBlockStore::default();
        let content_store = MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let _ = _init_save_unlock(wrapping_key, &metadata_store, &content_store).await?;
        Ok(())
    }

    #[tokio::test]
    async fn history() -> Result<()> {
        let metadata_store = MemoryBlockStore::default();
        let content_store = MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let mut metadata = _init_save_unlock(wrapping_key, &metadata_store, &content_store).await?;
        let _history = metadata.history(&metadata_store).await?;
        Ok(())
    }

    #[tokio::test]
    async fn build_details() -> Result<()> {
        let metadata_store = MemoryBlockStore::default();
        let content_store = MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let metadata = _init_save_unlock(wrapping_key, &metadata_store, &content_store).await?;
        let _build_details = metadata.build_details(&metadata_store).await?;
        Ok(())
    }

    #[tokio::test]
    async fn add_read() -> Result<()> {
        let metadata_store = MemoryBlockStore::default();
        let content_store = MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let mut fs_metadata =
            _init_save_unlock(wrapping_key, &metadata_store, &content_store).await?;

        let cat_path = vec!["cat.txt".to_string()];
        let kitty_bytes = "hello kitty".as_bytes().to_vec();
        // Add a new file
        fs_metadata
            .write(
                &cat_path,
                &metadata_store,
                &content_store,
                kitty_bytes.clone(),
            )
            .await?;

        let new_kitty_bytes = fs_metadata
            .read(&cat_path, &metadata_store, &content_store)
            .await?;
        assert_eq!(kitty_bytes, new_kitty_bytes);

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn add_read_large() -> Result<()> {
        let metadata_store = MemoryBlockStore::default();
        let content_store = MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let mut fs_metadata =
            _init_save_unlock(wrapping_key, &metadata_store, &content_store).await?;

        let cat_path = vec!["cat.txt".to_string()];
        let kitty_bytes = vec![0u8; 1024 * 1024 * 10];
        // Add a new file
        fs_metadata
            .write(
                &cat_path,
                &metadata_store,
                &content_store,
                kitty_bytes.clone(),
            )
            .await?;

        let new_kitty_bytes = fs_metadata
            .read(&cat_path, &metadata_store, &content_store)
            .await?;
        assert_eq!(kitty_bytes, new_kitty_bytes);

        Ok(())
    }

    #[tokio::test]
    async fn add_mkdir_mv() -> Result<()> {
        let metadata_store = MemoryBlockStore::default();
        let content_store = MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;

        let mut fs_metadata =
            _init_save_unlock(wrapping_key, &metadata_store, &content_store).await?;

        // Add a new file
        let file_path = vec!["file".to_string()];
        let content = "loop de doop de dah".as_bytes().to_vec();
        fs_metadata
            .write(&file_path, &metadata_store, &content_store, content.clone())
            .await?;

        // Add a new dir
        let dir_path = vec!["dir".to_string()];
        fs_metadata.mkdir(&dir_path, &metadata_store).await?;
        let new_dir = fs_metadata
            .get_node(&dir_path, &metadata_store)
            .await?
            .expect("dir not found");
        match new_dir {
            PrivateNode::Dir(dir) => dir,
            _ => panic!("dir not found"),
        };
        // Move File into the dir
        let mv_file_path = vec!["dir".to_string(), "file".to_string()];
        fs_metadata
            .mv(&file_path, &mv_file_path, &metadata_store, &content_store)
            .await?;
        let new_file = fs_metadata
            .get_node(&mv_file_path, &metadata_store)
            .await?
            .expect("file not found");
        match new_file {
            PrivateNode::File(file) => file,
            _ => panic!("file not found"),
        };

        // Save the metadata
        fs_metadata.save(&metadata_store, &metadata_store).await?;
        let mut fs_metadata = FsMetadata::unlock(wrapping_key, &metadata_store).await?;
        // Make sure the original file is gone
        let file_node = fs_metadata.get_node(&file_path, &metadata_store).await?;
        if file_node.is_some() {
            panic!("file not deleted")
        };
        // Make sure the new file is there
        let new_file_node = fs_metadata.get_node(&mv_file_path, &metadata_store).await?;
        match new_file_node {
            Some(_) => (),
            None => panic!("file not found"),
        };
        // Read the file
        let new_file_content = fs_metadata
            .read(&mv_file_path, &metadata_store, &content_store)
            .await?;
        assert_eq!(content, new_file_content);

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn write_large_mkdir() -> Result<()> {
        let metadata_store = MemoryBlockStore::default();
        let content_store = MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let mut fs_metadata =
            _init_save_unlock(wrapping_key, &metadata_store, &content_store).await?;

        let file_bytes = vec![0u8; 1024 * 1024 * 50];
        let file_path = vec!["file".to_string()];
        let dir_path = vec!["dir".to_string()];

        fs_metadata
            .write(&file_path, &metadata_store, &metadata_store, file_bytes)
            .await?;
        fs_metadata.save(&metadata_store, &content_store).await?;
        let mut fs_metadata = FsMetadata::unlock(wrapping_key, &metadata_store).await?;

        fs_metadata.mkdir(&dir_path, &metadata_store).await?;
        fs_metadata.save(&metadata_store, &content_store).await?;
        let _fs_metadata = FsMetadata::unlock(wrapping_key, &metadata_store).await?;

        Ok(())
    }

    #[tokio::test]
    async fn add_rm_read() -> Result<()> {
        let metadata_store = MemoryBlockStore::default();
        let content_store = MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let mut fs_metadata =
            _init_save_unlock(wrapping_key, &metadata_store, &content_store).await?;

        let cat_path = vec!["cat.txt".to_string()];
        let kitty_bytes = "hello kitty".as_bytes().to_vec();
        // Add a new file
        fs_metadata
            .write(
                &cat_path,
                &metadata_store,
                &content_store,
                kitty_bytes.clone(),
            )
            .await?;

        // Remove
        fs_metadata.rm(&cat_path, &metadata_store).await?;

        let result = fs_metadata
            .read(&cat_path, &metadata_store, &content_store)
            .await;
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn add_write_read() -> Result<()> {
        let metadata_store = MemoryBlockStore::default();
        let content_store = MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let mut fs_metadata =
            _init_save_unlock(wrapping_key, &metadata_store, &content_store).await?;

        let cat_path = vec!["cat.txt".to_string()];
        let kitty_bytes = "hello kitty".as_bytes().to_vec();
        // Add a new file
        fs_metadata
            .write(
                &cat_path,
                &metadata_store,
                &content_store,
                kitty_bytes.clone(),
            )
            .await?;

        let new_kitty_bytes = fs_metadata
            .read(&cat_path, &metadata_store, &content_store)
            .await?;
        assert_eq!(kitty_bytes, new_kitty_bytes);
        let puppy_bytes = "hello puppy".as_bytes().to_vec();
        // Replace existing content
        fs_metadata
            .write(
                &cat_path,
                &metadata_store,
                &content_store,
                puppy_bytes.clone(),
            )
            .await?;

        let new_puppy_bytes = fs_metadata
            .read(&cat_path, &metadata_store, &content_store)
            .await?;
        assert_eq!(puppy_bytes, new_puppy_bytes);

        Ok(())
    }

    #[tokio::test]
    async fn all_functions() -> Result<()> {
        let metadata_store = MemoryBlockStore::default();
        let content_store = MemoryBlockStore::default();
        let wrapping_key = &EcEncryptionKey::generate().await?;
        let mut fs_metadata =
            _init_save_unlock(wrapping_key, &metadata_store, &content_store).await?;

        let cat_path = vec!["cat.txt".to_string()];
        let kitty_bytes = "hello kitty".as_bytes().to_vec();
        // Add a new file
        fs_metadata
            .write(
                &cat_path,
                &metadata_store,
                &content_store,
                kitty_bytes.clone(),
            )
            .await?;

        let new_kitty_bytes = fs_metadata
            .read(&cat_path, &metadata_store, &content_store)
            .await?;
        assert_eq!(kitty_bytes, new_kitty_bytes);

        let dog_path = vec!["dog.txt".to_string()];
        let puppy_bytes = "hello puppy".as_bytes().to_vec();

        // Move cat.txt to dog.txt
        fs_metadata
            .mv(&cat_path, &dog_path, &metadata_store, &content_store)
            .await?;
        // Replace existing content
        fs_metadata
            .write(
                &dog_path,
                &metadata_store,
                &content_store,
                puppy_bytes.clone(),
            )
            .await?;

        let new_puppy_bytes = fs_metadata
            .read(&dog_path, &metadata_store, &content_store)
            .await?;
        assert_eq!(puppy_bytes, new_puppy_bytes);

        Ok(())
    }
}
