use crate::value;
use thiserror::Error;
use wasm_bindgen::JsValue;

/// Configuration errors.
#[derive(Debug, Error)]
pub(crate) enum WasmError {
    #[error("LS failed on path: {:?}", .0)]
    LSFailure(Vec<String>),
    #[error("Failed to open remote endpoint: {}", .0)]
    RemoteFailure(String)
}

impl From<WasmError> for JsValue {
    fn from(value: WasmError) -> Self {
        value!(value.to_string())
    }
}