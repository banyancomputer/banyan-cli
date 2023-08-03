//! This crate contains modules which are compiled to WASM
#![warn(missing_debug_implementations, missing_docs, rust_2018_idioms)]
/// Expose API functionality
mod api;
/// Expose blockstore functionality
mod blockstore;
/// Expose Errors
mod error;
/// Fetch remote data
mod fetch;
/// Expose FS functionality
mod fs;
/// Misc utilities
mod utils;

use std::borrow::Borrow;

pub use api::Api as TombApi;
pub use blockstore::CarV2BlockStore as TombBlockStore;
pub use fs::Fs as TombFs;
use js_sys::{Array, Object, Reflect};
use std::convert::From;
use tomb_crypt::prelude::EcEncryptionKey;
pub use web_sys::CryptoKey;

use wasm_bindgen::prelude::*;

use crate::utils::JsResult;

#[wasm_bindgen]
pub struct TombWasm {
    wrapping_key: EcEncryptionKey,
    api: TombApi,
    fs: Option<TombFs>,
}

// TODO: Figure out correct error pattern

#[wasm_bindgen]
/// TombWasm exposes the functionality of Tomb in a WASM module
impl TombWasm {
    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen(constructor)]
    pub async fn new(web_wrapping_key: CryptoKey, api_endpoint: String) -> Self {
        // Convert the wrapping key
        let wrapping_key = EcEncryptionKey::from(web_wrapping_key);
        // Create a new api
        let api = TombApi::new(api_endpoint);
        // Ok
        Self {
            wrapping_key,
            api,
            fs: None,
        }
    }

    #[wasm_bindgen]
    pub async fn list_buckets(&self) -> JsResult<Array> {
        // Call the api
        let buckets = self.api.list_buckets().await.unwrap();
        // Convert the buckets into a JsValue
        let buckets = buckets
            .iter()
            .map(|bucket| {
                // Create a new object for this bucket
                let obj = Object::new();
                // Set the bucket name
                Reflect::set(&obj, &value!("name"), &value!(bucket.name.as_str())).unwrap();
                // Set the bucket id
                Reflect::set(&obj, &value!("id"), &value!(bucket.id.as_str())).unwrap();
                // Return the bucket
                obj
            })
            .collect::<Array>();
        // Ok
        Ok(buckets)
    }

    #[wasm_bindgen]
    pub async fn load_bucket(&mut self, bucket_id: String) -> JsResult<()> {
        // Call the api
        let (_bucket, vec) = self.api.load_bucket(bucket_id).await.unwrap();
        // Create a new blockstore
        let blockstore = TombBlockStore::from_vec(vec).await?;
        // Create a new fs
        let fs = TombFs::new(self.wrapping_key.borrow(), blockstore).await?;
        // Set the fs
        self.fs = Some(fs);
        // Ok
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn ls(&self, path: String) -> JsResult<Array> {
        // Break the path into parts
        let parts = path.split('/').collect::<Vec<&str>>();
        // Call the fs
        let entries = self.fs.as_ref().unwrap().ls(parts).await?;
        // Convert the entries into a JsValue
        Ok(entries)
    }
}
