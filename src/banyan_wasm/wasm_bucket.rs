use std::ops::Deref;

use wasm_bindgen::prelude::*;

use crate::banyan_common::banyan_api::models::bucket::Bucket;

/// WASM Compatible version of the Bucket struct
#[derive(Debug, Clone)]
#[wasm_bindgen]
pub struct WasmBucket(Bucket);

#[wasm_bindgen]
impl WasmBucket {
    /// Type of the Bucket
    #[wasm_bindgen(js_name = "bucketType")]
    pub fn bucket_type(&self) -> String {
        self.r#type.to_string()
    }

    /// Id of the Bucket
    pub fn id(&self) -> String {
        self.id.to_string()
    }

    /// Name of the Bucket
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Storage Class of the Bucket
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
