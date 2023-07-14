use crate::value;
use thiserror::Error;
use wasm_bindgen::JsValue;

/// Configuration errors.
#[derive(Debug, Error)]
pub(crate) enum WasmError {
    // #[error("Failed to deserialize WNFS components from filesystem")]
    // FS,
    // #[error("LS failed on path: {:?}", .0)]
    // LS(Vec<String>),
    #[error("Failed to open remote endpoint: {}", .0)]
    Remote(String),
}

impl From<WasmError> for JsValue {
    fn from(value: WasmError) -> Self {
        value!(value.to_string())
    }
}
