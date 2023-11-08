//! This crate contains modules which are compiled to WASM
/// Compatibility
mod compat;

/// Expose all the compatibility types directly
pub use compat::{
    TombResult, TombWasm, TombWasmError, WasmBucket, WasmBucketKey, WasmBucketMetadata,
    WasmFsMetadataEntry, WasmMount, WasmNodeMetadata, WasmSnapshot,
};

/// Turn a value into a JsValue
#[macro_export]
macro_rules! value {
    ($value:expr) => {
        wasm_bindgen::JsValue::from($value)
    };
}

#[cfg(feature = "console_error_panic_hook")]
pub(crate) fn set_panic_hook() {
    console_error_panic_hook::set_once();
}
