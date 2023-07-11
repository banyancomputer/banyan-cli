use crate::value;
use js_sys::{Object, Reflect};
use wasm_bindgen::JsValue;
use wnfs::{common::Metadata, libipld::Ipld};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

pub(crate) struct JsMetadata<'a>(pub(crate) &'a Metadata);

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl TryFrom<JsMetadata<'_>> for JsValue {
    type Error = js_sys::Error;

    fn try_from(value: JsMetadata<'_>) -> Result<Self, Self::Error> {
        let metadata = Object::new();

        if let Some(Ipld::Integer(i)) = value.0 .0.get("created") {
            Reflect::set(
                &metadata,
                &value!("created"),
                &value!(i64::try_from(*i).unwrap() as f64),
            )?;
        }

        if let Some(Ipld::Integer(i)) = value.0 .0.get("modified") {
            Reflect::set(
                &metadata,
                &value!("modified"),
                &value!(i64::try_from(*i).unwrap() as f64),
            )?;
        }

        Ok(value!(metadata))
    }
}
