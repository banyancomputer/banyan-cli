use js_sys::{Object, Reflect};
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use tomb_common::keys::manager::Manager; // , utils::serialize::load_all};
use tomb_crypt::prelude::*;
use wasm_bindgen::JsValue;
use wnfs::{
    common::Metadata as WnfsMetadata,
    libipld::Ipld,
    private::{PrivateDirectory, PrivateForest},
};
use tomb_common::blockstore::{carv2_memory::CarV2MemoryBlockStore as BlockStore};
use crate::{error::TombWasmError, value};

pub struct SnapshotEntry<'a>(pub(crate) &'a WnfsMetadata);

// TODO: replace these with the types from banyan-api-client

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnapshotMetadata {
    pub id: String,
    pub bucket_id: String,
    pub snapshot_type: String,
    pub version: String,
}

pub struct Snapshot {
    pub snapshot_metadata: SnapshotMetadata,
    pub locked: bool,

    /* Fs Exposure  */
    // Encrypted metadata
    metadata: BlockStore,

    // TODO: Do we need to use Rc here?
    // TODO: Should I clone the wrapping key here?
    // Initialized Fs Entry
    metadata_forest: Option<Rc<PrivateForest>>,
    content_forest: Option<Rc<PrivateForest>>,
    dir: Option<Rc<PrivateDirectory>>,

    // Key Manager
    manager: Option<Manager>,
}

impl Snapshot {
    pub fn new(snapshot_metadata: SnapshotMetadata, metadata: BlockStore) -> Self {
        Self {
            // Snapshot Metadata
            snapshot_metadata,
            locked: true,

            // Encrypted metadata
            metadata,

            // Unlocked Snapshot Fs
            metadata_forest: None,
            content_forest: None,
            dir: None,
            manager: None,
        }
    }

    pub async fn unlock(&mut self, _wrapping_key: &EcEncryptionKey) -> Result<(), TombWasmError> {
        // Load all the components
        // let components = load_all(wrapping_key, &self.metadata)
        //     .await
        //     .map_err(TombWasmError::bucket_error)?;
        // let (metadata_forest, content_forest, dir, manager, _) = components;
        // // Set the components
        // self.metadata_forest = Some(metadata_forest);
        // self.content_forest = Some(content_forest);
        // self.dir = Some(dir);
        // self.manager = Some(manager);
        // self.locked = false;
        // Ok
        Ok(())
    }

    pub async fn ls(
        &self,
        path_segments: Vec<String>,
    ) -> Result<Vec<(String, WnfsMetadata)>, TombWasmError> {
        let dir = self.get_dir();
        let metadata_forest = self.get_metadata_forest();
        let entries = dir
            .ls(
                path_segments.as_slice(),
                true,
                metadata_forest,
                &self.metadata,
            )
            .await
            .map_err(TombWasmError::bucket_error)?;
        Ok(entries)
    }

    pub async fn share_with(
        &mut self,
        _recipient_key: &EcPublicEncryptionKey,
        _wrapping_key: &EcEncryptionKey,
    ) -> Result<(), TombWasmError> {
        panic!("not implemented")
    }

    // Internal Getters

    fn get_dir(&self) -> &Rc<PrivateDirectory> {
        self.dir
            .as_ref()
            .unwrap_or_else(|| panic!("Snapshot is locked"))
    }
    fn get_metadata_forest(&self) -> &Rc<PrivateForest> {
        self.metadata_forest.as_ref().unwrap()
    }
    fn get_content_forest(&self) -> &Rc<PrivateForest> {
        self.content_forest.as_ref().unwrap()
    }
    fn get_manager(&self) -> &Manager {
        self.manager.as_ref().unwrap()
    }
}

impl TryFrom<SnapshotMetadata> for JsValue {
    type Error = js_sys::Error;
    fn try_from(snapshot_metadata: SnapshotMetadata) -> Result<Self, Self::Error> {
        let metadata = Object::new();
        Reflect::set(&metadata, &value!("id"), &value!(snapshot_metadata.id))?;
        Reflect::set(
            &metadata,
            &value!("bucket_id"),
            &value!(snapshot_metadata.bucket_id),
        )?;
        Reflect::set(
            &metadata,
            &value!("snapshot_type"),
            &value!(snapshot_metadata.snapshot_type),
        )?;
        Reflect::set(
            &metadata,
            &value!("version"),
            &value!(snapshot_metadata.version),
        )?;
        Ok(value!(metadata))
    }
}

impl TryFrom<SnapshotEntry<'_>> for JsValue {
    type Error = js_sys::Error;
    fn try_from(fs_entry: SnapshotEntry<'_>) -> Result<Self, Self::Error> {
        let metadata = Object::new();
        if let Some(Ipld::Integer(i)) = fs_entry.0 .0.get("created") {
            Reflect::set(
                &metadata,
                &value!("created"),
                &value!(i64::try_from(*i).unwrap() as f64),
            )?;
        }
        if let Some(Ipld::Integer(i)) = fs_entry.0 .0.get("modified") {
            Reflect::set(
                &metadata,
                &value!("modified"),
                &value!(i64::try_from(*i).unwrap() as f64),
            )?;
        }
        Ok(value!(metadata))
    }
}
