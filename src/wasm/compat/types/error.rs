use std::error::Error;
use std::fmt::{self, Display, Formatter};
use wasm_bindgen::JsValue;

#[derive(Debug)]
pub struct TombWasmError(String);

impl TombWasmError {
    pub fn new(message: &str) -> TombWasmError {
        TombWasmError(String::from(message))
    }
}

impl Display for TombWasmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "tomm-wasm (unexpected error): {}", self.0)
    }
}

impl From<TombWasmError> for js_sys::Error {
    fn from(err: TombWasmError) -> Self {
        JsValue::from(err.0).into()
    }
}

impl Error for TombWasmError {}

pub fn to_js_error_with_debug<E: Error>(message: &str) -> impl Fn(E) -> js_sys::Error + '_ {
    move |err| js_sys::Error::from(TombWasmError(format!("{} | {}", message, err)))
}

pub fn to_wasm_error_with_debug<E: Error>(message: &str) -> impl Fn(E) -> TombWasmError + '_ {
    move |err| TombWasmError(format!("{} | {}", message, err))
}
