use std::error::Error;
use std::fmt::{self, Display, Formatter};

use wasm_bindgen::JsValue;

#[derive(Debug)]
#[non_exhaustive]
pub struct TombWasmError(pub String);

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
