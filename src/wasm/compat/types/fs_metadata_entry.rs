use crate::{
    filesystem::{FsMetadataEntry, FsMetadataEntryType},
    value,
    wasm::{TombWasmError, WasmNodeMetadata},
};
use js_sys::{Object, Reflect};
use wasm_bindgen::prelude::{JsCast, JsValue};

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

        let metadata = WasmNodeMetadata(fs_entry.0.metadata);

        let object = Object::new();

        Reflect::set(
            &object,
            &JsValue::from_str("name"),
            &JsValue::from_str(&name),
        )
        .expect("we know this is an object");
        Reflect::set(
            &object,
            &JsValue::from_str("type"),
            &JsValue::from_str(entry_type),
        )
        .expect("we know this is an object");
        Reflect::set(
            &object,
            &JsValue::from_str("metadata"),
            &JsValue::try_from(metadata)?,
        )
        .expect("we know this is an object");

        Ok(value!(object))
    }
}

impl TryFrom<JsValue> for WasmFsMetadataEntry {
    type Error = TombWasmError;

    fn try_from(js_value: JsValue) -> Result<Self, Self::Error> {
        let object = js_value.dyn_into::<Object>().map_err(|obj| {
            TombWasmError(format!(
                "expected object in version to WasmFsMetadataEntry: {obj:?}"
            ))
        })?;

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

        let metadata: WasmNodeMetadata = metadata_obj.try_into().map_err(|err| {
            TombWasmError(format!(
                "unable to parse object into bucket metadata: {err}"
            ))
        })?;

        Ok(Self(FsMetadataEntry {
            name,
            entry_type,
            metadata: metadata.0,
        }))
    }
}
