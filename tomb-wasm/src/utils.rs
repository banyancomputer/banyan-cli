use wasm_bindgen::prelude::*;
use js_sys::{
    Array, Error, Object, Reflect,
};
use std::{
    fmt::Debug
};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

pub type JsResult<T> = Result<T, js_sys::Error>;

//--------------------------------------------------------------------------------------------------
// Macro Definitions
//--------------------------------------------------------------------------------------------------

#[macro_export]
macro_rules! value {
    ($value:expr) => {
        wasm_bindgen::JsValue::from($value)
    };
}

//--------------------------------------------------------------------------------------------------
// Development Utilities
//--------------------------------------------------------------------------------------------------

#[wasm_bindgen(js_name = "setPanicHook")]
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

//--------------------------------------------------------------------------------------------------
// Error and Result Utilities
//--------------------------------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) fn error<E>(message: &str) -> impl FnOnce(E) -> Error + '_
where
    E: Debug,
{
    move |e| Error::new(&format!("{message}: {e:?}"))
}

#[allow(dead_code)]
pub(crate) fn anyhow_error<E>(message: &str) -> impl FnOnce(E) -> anyhow::Error + '_
where
    E: Debug,
{
    move |e| anyhow::Error::msg(format!("{message}: {e:?}"))
}

//--------------------------------------------------------------------------------------------------
// JS to Rust Conversion Utilities
//--------------------------------------------------------------------------------------------------

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

//--------------------------------------------------------------------------------------------------
// Misc Functions
//--------------------------------------------------------------------------------------------------

#[allow(dead_code)]
pub(crate) fn convert_path_segments(path_segments: &Array) -> JsResult<Vec<String>> {
    map_to_rust_vec(path_segments, |v| {
        v.as_string()
            .ok_or_else(|| Error::new("Invalid path segments: Expected an array of strings"))
    })
}