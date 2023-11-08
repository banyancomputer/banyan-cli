use crate::{value, wasm::TombWasmError};
use gloo::console::log;
use js_sys::{Object, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::{JsCast, JsValue};
use wnfs::{common::Metadata as NodeMetadata, libipld::Ipld};

pub struct WasmNodeMetadata(pub(crate) NodeMetadata);

impl TryFrom<JsValue> for WasmNodeMetadata {
    type Error = TombWasmError;

    fn try_from(js_value: JsValue) -> Result<Self, Self::Error> {
        let object = js_value
            .dyn_into::<Object>()
            .map_err(|_| TombWasmError("expected an object to be passed in".to_string()))?;

        let mut map = BTreeMap::new();

        // We know object is an Object already, so this shouldn't be able to panic (that is the
        // only documented way for this to throw an error).
        let created_ref =
            Reflect::get(&object, &JsValue::from_str("created")).expect("undocumented error");
        if let Some(timestamp) = created_ref.as_f64() {
            map.insert("created".into(), Ipld::Integer(timestamp as i128));
        } else {
            log!("WARNING: WasmNodeMetadata did not contain a 'created' timestamp");
        }

        // See created
        let modified_ref =
            Reflect::get(&object, &JsValue::from_str("created")).expect("undocumented error");
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

        if let Some(Ipld::Integer(i)) = fs_entry.0 .0.get("created") {
            Reflect::set(&object, &JsValue::from_str("created"), &value!(*i as f64))?;
        }

        if let Some(Ipld::Integer(i)) = fs_entry.0 .0.get("modified") {
            Reflect::set(&object, &JsValue::from_str("modified"), &value!(*i as f64))?;
        }

        if let Some(Ipld::String(s)) = fs_entry.0 .0.get("mime_type") {
            Reflect::set(&object, &JsValue::from_str("mime_type"), &value!(s))?;
        }

        if let Some(Ipld::Integer(i)) = fs_entry.0 .0.get("size") {
            Reflect::set(&object, &JsValue::from_str("size"), &value!(*i as f64))?;
        }

        Reflect::set(
            &object,
            &JsValue::from_str("cid"),
            &JsValue::from_str("Qmabcde"),
        )?;

        Ok(value!(object))
    }
}
