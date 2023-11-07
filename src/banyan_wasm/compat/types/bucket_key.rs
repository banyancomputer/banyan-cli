use std::ops::Deref;
use wasm_bindgen::prelude::*;
use crate::banyan_api::models::bucket_key::BucketKey;

/// WASM Compatible version of the BucketKey struct
#[derive(Debug)]
#[wasm_bindgen]
pub struct WasmBucketKey(pub(crate) BucketKey);

#[wasm_bindgen]
impl WasmBucketKey {
    /// The approval status of the Bucket Key
    pub fn approved(&self) -> bool {
        self.0.approved
    }

    /// The Bucket ID of the Bucket Key
    #[wasm_bindgen(js_name = "bucketId")]
    pub fn bucket_id(&self) -> String {
        self.0.bucket_id.to_string()
    }

    /// The ID of the Bucket Key
    pub fn id(&self) -> String {
        self.0.id.to_string()
    }

    /// The PEM of the Bucket Key
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
