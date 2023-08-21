use js_sys::{Object, Reflect};
use wnfs::{common::Metadata as FsEntryMetadata, libipld::Ipld};
use tomb_common::banyan::models::{
    bucket::*,
    bucket_key::*,
    snapshot::*,
};
use crate::value;
use wasm_bindgen::prelude::*;

/* Js Value bindings for our Rust structs */
// #[wasm_bindgen]
// pub struct WasmBucketType(pub(crate) BucketType);
// impl From<BucketType> for WasmBucketType {
//     fn from(bucket_type: BucketType) -> Self {
//         Self(bucket_type)
//     }
// }
// impl From<WasmBucketType> for BucketType {
//     fn from(wasm_bucket_type: WasmBucketType) -> Self {
//         wasm_bucket_type.0
//     }
// }
// #[wasm_bindgen]
// pub struct WasmStorageClass(pub(crate) StorageClass);
// impl From<StorageClass> for WasmStorageClass {
//     fn from(storage_class: StorageClass) -> Self {
//         Self(storage_class)
//     }
// }
// impl From<WasmStorageClass> for StorageClass {
//     fn from(wasm_storage_class: WasmStorageClass) -> Self {
//         wasm_storage_class.0
//     }
// }


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
pub struct WasmBucketEntry(pub(crate) FsEntryMetadata);
impl TryFrom<WasmBucketEntry> for JsValue {
    type Error = js_sys::Error;
    fn try_from(fs_entry: WasmBucketEntry) -> Result<Self, Self::Error> {
        let object  = Object::new();
        if let Some(Ipld::Integer(i)) = fs_entry.0 .0.get("created") {
            Reflect::set(
                &object ,
                &value!("created"),
                &value!(i64::try_from(*i).unwrap() as f64),
            )?;
        }
        if let Some(Ipld::Integer(i)) = fs_entry.0 .0.get("modified") {
            Reflect::set(
                &object ,
                &value!("modified"),
                &value!(i64::try_from(*i).unwrap() as f64),
            )?;
        }
        // TODO: Remove stubs, with standard object 
        Reflect::set(&object , &value!("size"), &value!(1024))?;
        Reflect::set(&object , &value!("cid"), &value!("Qmabcde"))?;
        Ok(value!(object ))
    }
}

#[wasm_bindgen]
/// A wrapper around a snapshot
pub struct WasmSnapshot(pub(crate) Snapshot);