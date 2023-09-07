use std::ops::Deref;

use wasm_bindgen::prelude::*;

use tomb_common::banyan_api::models::bucket::Bucket;

#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct WasmBucket(Bucket);

#[wasm_bindgen]
impl WasmBucket {
    #[wasm_bindgen(js_name = "bucketType")]
    pub fn bucket_type(&self) -> String {
        self.r#type.to_string()
    }

    pub fn id(&self) -> String {
        self.id.to_string()
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[wasm_bindgen(js_name = "storageClass")]
    pub fn storage_class(&self) -> String {
        self.storage_class.to_string()
    }
}

impl From<Bucket> for WasmBucket {
    fn from(bucket: Bucket) -> Self {
        Self(bucket)
    }
}

impl From<WasmBucket> for Bucket {
    fn from(wasm_bucket: WasmBucket) -> Self {
        wasm_bucket.0
    }
}

impl Deref for WasmBucket {
    type Target = Bucket;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
