use js_sys::{Array, Error, Uint8Array};
use wasm_bindgen::prelude::*;

pub type JsResult<T> = Result<T, js_sys::Error>;

/// Turn a value into a JsValue
#[macro_export]
macro_rules! value {
    ($value:expr) => {
        wasm_bindgen::JsValue::from($value)
    };
}

pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

pub(crate) fn map_to_rust_vec<T, F: FnMut(JsValue) -> JsResult<T>>(
    array: &Array,
    f: F,
) -> JsResult<Vec<T>> {
    array
        .to_vec()
        .into_iter()
        .map(f)
        .collect::<JsResult<Vec<_>>>()
}

#[inline]
#[allow(dead_code)]
/// Convert Vec of bytes to JsResult of bytes with known length
pub(crate) fn expect_bytes<const N: usize>(bytes: Vec<u8>) -> JsResult<[u8; N]> {
    bytes.try_into().map_err(|v: Vec<u8>| {
        Error::new(&format!(
            "Unexpected number of bytes received. Expected {N}, but got {}",
            v.len()
        ))
    })
}

#[allow(dead_code)]
pub(crate) fn convert_path_segments(path_segments: &Array) -> JsResult<Vec<String>> {
    map_to_rust_vec(path_segments, |v| {
        v.as_string()
            .ok_or_else(|| Error::new("Invalid path segments: Expected an array of strings"))
    })
}