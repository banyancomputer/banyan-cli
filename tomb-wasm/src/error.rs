use std::fmt::{self, Display, Formatter};

use wasm_bindgen::JsValue;

#[derive(Debug)]
#[non_exhaustive]
pub struct TombWasmError {
    kind: TombWasmErrorKind,
}

impl TombWasmError {
    pub fn unknown_error() -> Self {
        Self {
            kind: TombWasmErrorKind::UnknownError,
        }
    }
}

impl Display for TombWasmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use TombWasmErrorKind::*;

        match &self.kind {
            UnknownError => write!(f, "an unknown error occurred")
        }
    }
}

impl From<TombWasmError> for js_sys::Error {
    fn from(err: TombWasmError) -> Self {
        JsValue::from("an unknown error occurred".to_string()).into()
    }
}

impl std::error::Error for TombWasmError {}

#[derive(Debug)]
#[non_exhaustive]
enum TombWasmErrorKind {
    UnknownError
}
