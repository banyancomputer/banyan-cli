use wasm_bindgen::prelude::wasm_bindgen;
use super::{WasmBucket, WasmMount};

#[wasm_bindgen]
pub struct WasmBucketMount {
    bucket: WasmBucket, 
    mount: WasmMount 
}

#[wasm_bindgen]
impl WasmBucketMount {

    #[wasm_bindgen]
    pub fn new(bucket: WasmBucket, mount: WasmMount) -> WasmBucketMount {
        WasmBucketMount { bucket, mount }
    }

    #[wasm_bindgen(getter)]
    pub fn bucket(&self) -> WasmBucket {
        self.bucket.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn mount(&self) -> WasmMount {
        self.mount.clone()
    }
}