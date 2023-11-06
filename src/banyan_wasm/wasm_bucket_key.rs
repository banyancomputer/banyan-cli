use std::ops::Deref;

use wasm_bindgen::prelude::*;

use tomb_common::banyan_api::models::bucket_key::BucketKey;

#[wasm_bindgen]
pub struct WasmBucketKey(pub(crate) BucketKey);

#[wasm_bindgen]
impl WasmBucketKey {
    pub fn approved(&self) -> bool {
        self.0.approved
    }

    #[wasm_bindgen(js_name = "bucketId")]
    pub fn bucket_id(&self) -> String {
        self.0.bucket_id.to_string()
    }

    pub fn id(&self) -> String {
        self.0.id.to_string()
    }

    pub fn pem(&self) -> String {
        self.0.pem.clone()
    }
}

impl Deref for WasmBucketKey {
    type Target = BucketKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<WasmBucketKey> for BucketKey {
    fn from(wasm_bucket_key: WasmBucketKey) -> Self {
        wasm_bucket_key.0
    }
}
