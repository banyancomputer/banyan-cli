use wasm_bindgen::prelude::*;
use crate::banyan_api::models::metadata::Metadata;

/// A wrapper around a Bucket Metadata
#[derive(Debug)]
#[wasm_bindgen]
pub struct WasmBucketMetadata(pub(crate) Metadata);

/// Getters
#[wasm_bindgen]
impl WasmBucketMetadata {
    /// Metadata ID
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.0.id.to_string()
    }

    /// Bucket ID
    #[wasm_bindgen(getter = bucketId)]
    pub fn bucket_id(&self) -> String {
        self.0.bucket_id.to_string()
    }

    /// Snapshot ID
    #[wasm_bindgen(getter = snapshotId)]
    pub fn snapshot_id(&self) -> String {
        self.0.snapshot_id.expect("no snapshot").to_string()
    }
}