use js_sys::{Array, Error, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_streams::ReadableStream;

pub type JsResult<T> = Result<T, js_sys::Error>;

/// Turn a value into a JsValue
#[macro_export]
macro_rules! value {
    ($value:expr) => {
        wasm_bindgen::JsValue::from($value)
    };
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

/// Read a Vec<u8> from a ReadableStream
/// # Arguments
/// * `url` - A string slice that holds the URL to fetch
pub(crate) async fn read_vec_from_readable_stream(
    stream: &mut ReadableStream,
) -> JsResult<Vec<u8>> {
    let mut reader = stream.get_reader();
    let mut data: Vec<u8> = vec![];
    while let Ok(Some(result)) = reader.read().await {
        let chunk = Uint8Array::from(result);
        data.extend(chunk.to_vec());
    }
    Ok(data)
}
