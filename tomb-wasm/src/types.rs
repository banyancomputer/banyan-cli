use std::collections::BTreeMap;

use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::*;
use wnfs::{common::Metadata as NodeMetadata, libipld::Ipld};

use tomb_common::banyan_api::models::metadata::*;
use tomb_common::banyan_api::models::snapshot::*;
use tomb_common::metadata::{FsMetadataEntry, FsMetadataEntryType};

use crate::{log, value};
use crate::error::TombWasmError;

pub struct WasmNodeMetadata(pub(crate) NodeMetadata);

impl TryFrom<JsValue> for WasmNodeMetadata {
    type Error = TombWasmError;

    fn try_from(js_value: JsValue) -> Result<Self, Self::Error> {
        let object = js_value.dyn_into::<Object>()
            .map_err(|_| TombWasmError(format!("expected an object to be passed in")))?;

        let mut map = BTreeMap::new();

        // We know object is an Object already, so this shouldn't be able to panic (that is the
        // only documented way for this to throw an error).
        let created_ref = Reflect::get(&object, &JsValue::from_str("created"))
            .expect("undocumented error");
        if let Some(timestamp) = created_ref.as_f64() {
            map.insert("created".into(), Ipld::Integer(timestamp as i128));
        } else {
            log!("WARNING: WasmNodeMetadata did not contain a 'created' timestamp");
        }

        // See created
        let modified_ref = Reflect::get(&object, &JsValue::from_str("created"))
            .expect("undocumented error");
        if let Some(timestamp) = modified_ref.as_f64() {
            map.insert("modified".into(), Ipld::Integer(timestamp as i128));
        } else {
            log!("WARNING: WasmNodeMetadata did not contain a 'modified' timestamp");
        }

        Ok(Self(NodeMetadata(map)))
    }
}

impl TryFrom<WasmNodeMetadata> for JsValue {
    type Error = js_sys::Error;

    fn try_from(fs_entry: WasmNodeMetadata) -> Result<Self, Self::Error> {
        let object = Object::new();

        if let Some(Ipld::Integer(i)) = fs_entry.0.0.get("created") {
            Reflect::set(
                &object,
                &JsValue::from_str("created"),
                &value!(*i as f64),
            )?;
        }

        if let Some(Ipld::Integer(i)) = fs_entry.0.0.get("modified") {
            Reflect::set(
                &object,
                &JsValue::from_str("modified"),
                &value!(*i as f64),
            )?;
        }

        // TODO: Remove stubs, with standard object
        Reflect::set(&object, &JsValue::from_str("size"), &JsValue::from_f64(1024.0))?;
        Reflect::set(&object, &JsValue::from_str("cid"), &JsValue::from_str("Qmabcde"))?;

        Ok(value!(object))
    }
}

#[wasm_bindgen]
pub struct WasmSnapshot(Snapshot);

#[wasm_bindgen]
impl WasmSnapshot {
    pub fn id(&self) -> String {
        self.0.id.to_string()
    }

    #[wasm_bindgen(js_name = "bucketId")]
    pub fn bucket_id(&self) -> String {
        self.0.bucket_id.to_string()
    }

    #[wasm_bindgen(js_name = "metadataId")]
    pub fn metadata_id(&self) -> String {
        self.0.metadata_id.to_string()
    }

    pub(crate) fn new(snapshot: Snapshot) -> Self {
        Self(snapshot)
    }

    #[wasm_bindgen(js_name = "dataSize")]
    pub fn created_at(&self) -> i64 {
        self.0.created_at
    }
}

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
    pub fn entry_type(&self) -> String {
        match self.0.entry_type {
            FsMetadataEntryType::File => "file".to_string(),
            FsMetadataEntryType::Dir => "dir".to_string(),
        }
    }

    pub fn metadata(&self) -> WasmNodeMetadata {
        WasmNodeMetadata(self.0.metadata.clone())
    }

    pub fn name(&self) -> String {
        self.0.name.clone()
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

        let metadata = WasmNodeMetadata(fs_entry.0.metadata.clone());

        let object = Object::new();

        Reflect::set(&object, &JsValue::from_str("name"), &JsValue::from_str(&name)).expect("we know this is an object");
        Reflect::set(&object, &JsValue::from_str("type"), &JsValue::from_str(entry_type)).expect("we know this is an object");
        Reflect::set(&object, &JsValue::from_str("metadata"), &JsValue::try_from(metadata)?).expect("we know this is an object");

        Ok(value!(object))
    }
}

impl TryFrom<JsValue> for WasmFsMetadataEntry {
    type Error = TombWasmError;

    fn try_from(js_value: JsValue) -> Result<Self, Self::Error> {
        let object = js_value.dyn_into::<Object>()
            .map_err(|obj| { TombWasmError(format!("expected object in version to WasmFsMetadataEntry: {obj:?}")) })?;

        let name = Reflect::get(&object, &value!("name"))
            .expect("we know this is an object")
            .as_string()
            .ok_or(TombWasmError("name was not a string".into()))?;

        let type_str = Reflect::get(&object, &JsValue::from_str("type"))
            .expect("we know this is an object")
            .as_string()
            .ok_or(TombWasmError("type was not a string".into()))?;

        let entry_type = match type_str.as_str() {
            "dir" => FsMetadataEntryType::Dir,
            "file" => FsMetadataEntryType::File,
            _ => return Err(TombWasmError(format!("unknown entry type: {type_str}"))),
        };

        let metadata_obj = Reflect::get(&object, &JsValue::from_str("metadata"))
            .expect("we know this is an object");

        let metadata: WasmNodeMetadata = metadata_obj
            .try_into()
            .map_err(|err| TombWasmError(format!("unable to parse object into bucket metadata: {err}")))?;

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
        ).expect("we know this is an object");

        Reflect::set(
            &object,
            &value!("bucket_id"),
            &value!(bucket_metadata.0.bucket_id.to_string()),
        ).expect("we know this is an object");

        Reflect::set(
            &object,
            &value!("root_cid"),
            &value!(bucket_metadata.0.root_cid),
        ).expect("we know this is an object");

        Reflect::set(
            &object,
            &value!("metadata_cid"),
            &value!(bucket_metadata.0.metadata_cid),
        ).expect("we know this is an object");

        Reflect::set(
            &object,
            &value!("data_size"),
            &value!(bucket_metadata.0.data_size),
        ).expect("we know this is an object");

        Reflect::set(
            &object,
            &value!("state"),
            &value!(bucket_metadata.0.state.to_string()),
        ).expect("we know this is an object");

        Ok(value!(object))
    }
}
