use chrono::Utc;
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

use tomb_common::blockstore::carv2_memory::CarV2MemoryBlockStore as BlockStore;
use crate::{
    banyan::snapshot::Snapshot, error::TombWasmError,
    value,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketEntry(pub(crate) WnfsMetadata);

// TODO: replace these with the types from banyan-api-client

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BucketMetadata {
    pub id: String,
    // TODO: Should this be an enum? What types of buckets are there?
    pub bucket_type: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BucketKey {
    pub id: String,
    pub bucket_id: String,
    pub pem: String,
    pub approved: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BucketSnapshot {
    pub id: String,
    pub bucket_id: String,
    pub version: String,
}

pub struct Bucket {
    pub bucket_metadata: BucketMetadata,
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

impl Bucket {
    pub fn new(bucket_metadata: BucketMetadata, metadata: BlockStore) -> Self {
        Self {
            // Bucket Metadata
            bucket_metadata,
            locked: true,

            // Encrypted metadata
            metadata,

            // Unlocked Bucket Fs
            metadata_forest: None,
            content_forest: None,
            dir: None,
            manager: None,
        }
    }

    pub async fn unlock(&mut self) -> Result<(), TombWasmError> {
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
        self.locked = false;
        // Ok
        Ok(())
    }

    pub async fn ls(
        &self,
        _path_segments: Vec<&str>,
    ) -> Result<Vec<(String, BucketEntry)>, TombWasmError> {
        // let dir = self.get_dir();
        // let metadata_forest = self.get_metadata_forest();
        // let entries = dir
        //     .ls(
        //         path_segments.as_slice(),
        //         true,
        //         metadata_forest,
        //         &self.metadata,
        //     )
        //     .await
        //     .map_err(TombWasmError::bucket_error)?;
        // Ok(entries)
        // Return some sample data
        // file size
        // file cid or id
        Ok([
            (
                "puppy.png".to_string(),
                BucketEntry(WnfsMetadata::new(Utc::now())),
            ),
            (
                "chonker.jpg".to_string(),
                BucketEntry(WnfsMetadata::new(Utc::now())),
            ),
            (
                "floof_doof.mp3".to_string(),
                BucketEntry(WnfsMetadata::new(Utc::now())),
            ),
        ]
        .to_vec())
    }

    pub async fn share_with(
        &mut self,
        _recipient_key: &EcPublicEncryptionKey,
        _wrapping_key: &EcEncryptionKey,
    ) -> Result<(), TombWasmError> {
        panic!("not implemented")
    }

    pub async fn snapshot(&mut self) -> Result<Snapshot, TombWasmError> {
        panic!("not implemented")
    }

    // Getters
    pub fn is_locked(&self) -> bool {
        self.locked == true
    }

    fn get_dir(&self) -> &Rc<PrivateDirectory> {
        self.dir
            .as_ref()
            .unwrap_or_else(|| panic!("Bucket is locked"))
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

impl TryFrom<BucketMetadata> for JsValue {
    type Error = js_sys::Error;
    fn try_from(bucket_metadata: BucketMetadata) -> Result<Self, Self::Error> {
        let metadata = Object::new();
        Reflect::set(&metadata, &value!("id"), &value!(bucket_metadata.id))?;
        Reflect::set(
            &metadata,
            &value!("bucket_type"),
            &value!(bucket_metadata.bucket_type),
        )?;
        Reflect::set(&metadata, &value!("name"), &value!(bucket_metadata.name))?;
        Ok(value!(metadata))
    }
}

impl TryFrom<BucketKey> for JsValue {
    type Error = js_sys::Error;
    fn try_from(bucket_key: BucketKey) -> Result<Self, Self::Error> {
        let metadata = Object::new();
        Reflect::set(&metadata, &value!("id"), &value!(bucket_key.id))?;
        Reflect::set(
            &metadata,
            &value!("bucket_id"),
            &value!(bucket_key.bucket_id),
        )?;
        Reflect::set(&metadata, &value!("pem"), &value!(bucket_key.pem))?;
        Reflect::set(&metadata, &value!("approved"), &value!(bucket_key.approved))?;
        Ok(value!(metadata))
    }
}

impl TryFrom<BucketEntry> for JsValue {
    type Error = js_sys::Error;
    fn try_from(fs_entry: BucketEntry) -> Result<Self, Self::Error> {
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
        // TODO: Remove stubs, with standard metadata
        Reflect::set(&metadata, &value!("size"), &value!(1024))?;
        Reflect::set(&metadata, &value!("cid"), &value!("Qmabcde"))?;
        Ok(value!(metadata))
    }
}
