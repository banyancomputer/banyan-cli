use crate::{api::models::metadata::Metadata, wasm::TombWasmError};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct WasmBucketMetadata(pub(crate) Metadata);

#[wasm_bindgen]
impl WasmBucketMetadata {
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.0.id.to_string()
    }

    #[wasm_bindgen(getter = bucketId)]
    pub fn bucket_id(&self) -> String {
        self.0.bucket_id.to_string()
    }

    #[wasm_bindgen(getter = snapshotId)]
    pub fn snapshot_id(&self) -> String {
        self.0
            .snapshot_id
            .ok_or(TombWasmError::new("missing Snapshot id"))
            .unwrap()
            .to_string()
    }
}
