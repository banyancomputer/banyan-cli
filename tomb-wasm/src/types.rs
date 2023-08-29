use std::collections::BTreeMap;

use crate::value;
use js_sys::{Object, Reflect, Array};
use tomb_common::{
    banyan_api::models::{bucket::*, bucket_key::*, snapshot::*},
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

impl WasmBucket {
    pub fn name(&self) -> String {
        self.0.name.clone()
    }
    pub fn storage_class(&self) -> String {
        self.0.storage_class.clone().to_string()
    }
    pub fn bucket_type(&self) -> String {
        self.0.r#type.clone().to_string()
    }
    pub fn id(&self) -> String {
        self.0.id.clone().to_string()
    }
}

#[wasm_bindgen]
/// Wrapper around a BucketKey
pub struct WasmBucketKey(pub(crate) BucketKey);
impl From<BucketKey> for WasmBucketKey {
    fn from(bucket_key: BucketKey) -> Self {
        Self(bucket_key)
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
        let created = Reflect::get(&object, &value!("created"))?
            .as_f64()
            .unwrap() as i64;
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

#[wasm_bindgen]
/// A wrapper around a snapshot
pub struct WasmSnapshot(pub(crate) Snapshot);

#[derive(Clone)]
pub struct WasmFsMetadataEntry(pub(crate) FsMetadataEntry);
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
        let name = Reflect::get(&object, &value!("name"))?
            .as_string()
            .unwrap();
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