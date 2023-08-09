use std::fmt::{self, Display, Formatter};

use wasm_bindgen::JsValue;

#[derive(Debug)]
#[non_exhaustive]
pub struct TombWasmError {
    kind: TombWasmErrorKind,
}

impl TombWasmError {
    pub fn fetch_error(err: wasm_bindgen::JsValue) -> Self {
        Self {
            kind: TombWasmErrorKind::FetchError(err.into()),
        }
    }
    pub fn blockstore_error(err: js_sys::Error) -> Self {
        Self {
            kind: TombWasmErrorKind::BlockStoreError(err),
        }
    }
    pub fn bucket_error(err: anyhow::Error) -> Self {
        Self {
            kind: TombWasmErrorKind::BucketError(err),
        }
    }
    pub fn client_error(err: anyhow::Error) -> Self {
        Self {
            kind: TombWasmErrorKind::ClientError(err),
        }
    }
    pub fn car_error(err: String) -> Self {
        Self {
            kind: TombWasmErrorKind::BlockStoreError(JsValue::from(err).into()),
        }
    }
}

impl Display for TombWasmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use TombWasmErrorKind::*;

        match &self.kind {
            FetchError(err) => write!(f, "fetch error: {}", err.message()),
            BlockStoreError(err) => write!(f, "blockstore error: {}", err.message()),
            BucketError(err) => write!(f, "bucket error: {}", err.to_string()),
            ClientError(err) => write!(f, "client error: {}", err.to_string()),
        }
    }
}

impl From<TombWasmError> for js_sys::Error {
    fn from(err: TombWasmError) -> Self {
        use TombWasmErrorKind::*;

        match err.kind {
            FetchError(err) => err,
            BlockStoreError(err) => err,
            BucketError(err) => JsValue::from(err.to_string()).into(),
            ClientError(err) => JsValue::from(err.to_string()).into(),
        }
    }
}

impl std::error::Error for TombWasmError {}

#[derive(Debug)]
#[non_exhaustive]
enum TombWasmErrorKind {
    /// Error from Fetching a remote resource
    FetchError(js_sys::Error),
    /// Error from the blockstore
    BlockStoreError(js_sys::Error),
    /// Error from Bucket
    BucketError(anyhow::Error),
    /// Error from Fetching a remote resource using Client
    ClientError(anyhow::Error),
}
