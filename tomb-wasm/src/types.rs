use std::collections::BTreeMap;

use crate::value;
use js_sys::{Object, Reflect};
use tomb_common::{
    banyan_api::models::{bucket::*, bucket_key::*, metadata::*, snapshot::*},
    metadata::{FsMetadataEntry, FsMetadataEntryType},
};

use wasm_bindgen::prelude::*;
use wnfs::{common::Metadata as NodeMetadata, libipld::Ipld};

/// Wrapper around a Bucket
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct WasmBucket(pub(crate) Bucket);

impl From<Bucket> for WasmBucket {
    fn from(bucket: Bucket) -> Self {
        Self(bucket)
    }
}

impl From<WasmBucket> for Bucket {
    fn from(wasm_bucket: WasmBucket) -> Self {
        wasm_bucket.0
    }
}

#[wasm_bindgen]
impl WasmBucket {
    pub fn name(&self) -> String {
        self.0.name.clone()
    }

    #[wasm_bindgen(js_name = "storageClass")]
    pub fn storage_class(&self) -> String {
        self.0.storage_class.clone().to_string()
    }

    #[wasm_bindgen(js_name = "bucketType")]
    pub fn bucket_type(&self) -> String {
        self.0.r#type.clone().to_string()
    }

    pub fn id(&self) -> String {
        self.0.id.clone().to_string()
    }
}

/// Wrapper around a BucketKey
#[wasm_bindgen]
pub struct WasmBucketKey(pub(crate) BucketKey);
impl From<WasmBucketKey> for BucketKey {
    fn from(wasm_bucket_key: WasmBucketKey) -> Self {
        wasm_bucket_key.0
    }
}
#[wasm_bindgen]
impl WasmBucketKey {
    pub fn id(&self) -> String {
        self.0.id.to_string()
    }
    #[wasm_bindgen(js_name = "bucketId")]
    pub fn bucket_id(&self) -> String {
        self.0.bucket_id.to_string()
    }
    pub fn pem(&self) -> String {
        self.0.pem.clone()
    }
    pub fn approved(&self) -> bool {
        self.0.approved
    }
}

#[derive(Clone)]
pub struct WasmNodeMetadata(pub(crate) NodeMetadata);
impl TryFrom<WasmNodeMetadata> for JsValue {
    type Error = js_sys::Error;
    fn try_from(fs_entry: WasmNodeMetadata) -> Result<Self, Self::Error> {
        let object = Object::new();
        if let Some(Ipld::Integer(i)) = fs_entry.0 .0.get("created") {
            Reflect::set(
                &object,
                &value!("created"),
                &value!(i64::try_from(*i).unwrap() as f64),
            )?;
        }
        if let Some(Ipld::Integer(i)) = fs_entry.0 .0.get("modified") {
            Reflect::set(
                &object,
                &value!("modified"),
                &value!(i64::try_from(*i).unwrap() as f64),
            )?;
        }
        // TODO: Remove stubs, with standard object
        Reflect::set(&object, &value!("size"), &value!(1024))?;
        Reflect::set(&object, &value!("cid"), &value!("Qmabcde"))?;
        Ok(value!(object))
    }
}
impl TryFrom<JsValue> for WasmNodeMetadata {
    type Error = js_sys::Error;
    fn try_from(js_value: JsValue) -> Result<Self, Self::Error> {
        let object = js_value.dyn_into::<Object>()?;
        let created = Reflect::get(&object, &value!("created"))?.as_f64().unwrap() as i64;
        let modified = Reflect::get(&object, &value!("modified"))?
            .as_f64()
            .unwrap() as i64;
        let mut map = BTreeMap::new();
        map.insert("created".into(), Ipld::Integer(created as i128));
        map.insert("modified".into(), Ipld::Integer(modified as i128));
        let metadata = NodeMetadata(map);
        Ok(Self(metadata))
    }
}

/// A wrapper around a snapshot
#[wasm_bindgen]
pub struct WasmSnapshot(pub(crate) Snapshot);
impl From<Snapshot> for WasmSnapshot {
    fn from(snapshot: Snapshot) -> Self {
        Self(snapshot)
    }
}
impl From<WasmSnapshot> for Snapshot {
    fn from(wasm_snapshot: WasmSnapshot) -> Self {
        wasm_snapshot.0
    }
}
#[wasm_bindgen]
impl WasmSnapshot {
    pub fn id(&self) -> String {
        self.0.id.clone().to_string()
    }
    #[wasm_bindgen(js_name = "bucketId")]
    pub fn bucket_id(&self) -> String {
        self.0.bucket_id.clone().to_string()
    }
    #[wasm_bindgen(js_name = "metadataId")]
    pub fn metadata_id(&self) -> String {
        self.0.metadata_id.clone().to_string()
    }
    #[wasm_bindgen(js_name = "dataSize")]
    pub fn created_at(&self) -> i64 {
        self.0.created_at
    }
}

// TODO: Remove stubs
// TODO: Proper wasm bindings
#[derive(Clone)]
pub struct WasmFsMetadataEntry(pub(crate) FsMetadataEntry);
impl From<FsMetadataEntry> for WasmFsMetadataEntry {
    fn from(fs_metadata_entry: FsMetadataEntry) -> Self {
        Self(fs_metadata_entry)
    }
}
impl From<WasmFsMetadataEntry> for FsMetadataEntry {
    fn from(wasm_fs_metadata_entry: WasmFsMetadataEntry) -> Self {
        wasm_fs_metadata_entry.0
    }
}
impl WasmFsMetadataEntry {
    pub fn name(&self) -> String {
        self.0.name.clone()
    }
    pub fn entry_type(&self) -> String {
        match self.0.entry_type {
            FsMetadataEntryType::File => "file".to_string(),
            FsMetadataEntryType::Dir => "dir".to_string(),
        }
    }
    pub fn metadata(&self) -> WasmNodeMetadata {
        WasmNodeMetadata(self.0.metadata.clone())
    }
}
impl TryFrom<WasmFsMetadataEntry> for JsValue {
    type Error = js_sys::Error;
    fn try_from(fs_entry: WasmFsMetadataEntry) -> Result<Self, Self::Error> {
        let name = fs_entry.0.name.clone();
        let entry_type = match fs_entry.0.entry_type {
            FsMetadataEntryType::File => "file",
            FsMetadataEntryType::Dir => "dir",
        };
        let metadata: WasmNodeMetadata = WasmNodeMetadata(fs_entry.0.metadata.clone());
        let object = Object::new();
        Reflect::set(&object, &value!("name"), &value!(name))?;
        Reflect::set(&object, &value!("type"), &value!(entry_type))?;
        Reflect::set(&object, &value!("metadata"), &JsValue::try_from(metadata)?)?;
        Ok(value!(object))
    }
}
impl TryFrom<JsValue> for WasmFsMetadataEntry {
    type Error = js_sys::Error;
    fn try_from(js_value: JsValue) -> Result<Self, Self::Error> {
        let object = js_value.dyn_into::<Object>()?;
        let name = Reflect::get(&object, &value!("name"))?.as_string().unwrap();
        let entry_type = match Reflect::get(&object, &value!("type"))?
            .as_string()
            .unwrap()
            .as_str()
        {
            "file" => FsMetadataEntryType::File,
            "dir" => FsMetadataEntryType::Dir,
            _ => panic!("Invalid FsMetadataEntryType"),
        };
        let metadata: WasmNodeMetadata = Reflect::get(&object, &value!("metadata"))?.try_into()?;
        Ok(Self(FsMetadataEntry {
            name,
            entry_type,
            metadata: metadata.0,
        }))
    }
}

/// A wrapper around a bucket metadata
pub struct WasmBucketMetadata(pub(crate) Metadata);

impl TryFrom<WasmBucketMetadata> for JsValue {
    type Error = js_sys::Error;
    fn try_from(bucket_metadata: WasmBucketMetadata) -> Result<Self, Self::Error> {
        let object = Object::new();
        Reflect::set(
            &object,
            &value!("id"),
            &value!(bucket_metadata.0.id.to_string()),
        )?;
        Reflect::set(
            &object,
            &value!("bucket_id"),
            &value!(bucket_metadata.0.bucket_id.to_string()),
        )?;
        Reflect::set(
            &object,
            &value!("root_cid"),
            &value!(bucket_metadata.0.root_cid),
        )?;
        Reflect::set(
            &object,
            &value!("metadata_cid"),
            &value!(bucket_metadata.0.metadata_cid),
        )?;
        Reflect::set(
            &object,
            &value!("data_size"),
            &value!(bucket_metadata.0.data_size),
        )?;
        Reflect::set(
            &object,
            &value!("state"),
            &value!(bucket_metadata.0.state.to_string()),
        )?;
        Ok(value!(object))
    }
}
