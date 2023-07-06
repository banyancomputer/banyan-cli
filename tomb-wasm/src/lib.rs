// Crate Modules
mod fetch;
mod metadata;
mod utils;

// use crate::utils::JsResult;
use crate::metadata::types::{ Service as MetadataService, Bucket};

// WASM Imports
use wasm_bindgen::prelude::*;
// use wasm_bindgen_futures::future_to_promise;
use gloo::console::log;
// use js_sys::{
//     Array, Promise
// };

// WNFS Imports
use wnfs::{
    common::MemoryBlockStore,
    private::{
        PrivateDirectory,
        PrivateForest,
        TemporalKey, KEY_BYTE_SIZE,
    },
    namefilter::Namefilter
};

// Tomb Imports
use tomb_common::crypto::rsa::{
    PrivateKey, ExchangeKey,
    RsaPrivateKey, RsaPublicKey
};

// Std / Misc Imports
use rand::thread_rng;
use std::rc::Rc;
use chrono::Utc;

// Our optional WeeAlloc allocator
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[allow(dead_code)]
#[wasm_bindgen]
pub struct Tomb {
    buckets: Vec<Bucket>,
    metadata_service: MetadataService,
    // TODO: @vera this should be replaced with CarV2MemoryBlockStore
    blockstore: MemoryBlockStore,
    // TODO: unclear to me whether these need to be wrapped in Rc 
    // private_forest: Option<Rc<PrivateForest>>,
    // private_directory: Option<Rc<PrivateDirectory>>
    private_forest: Option<PrivateForest>,
    private_directory: Option<PrivateDirectory>
}

// Public methods, exported to JavaScript.
#[wasm_bindgen]
impl Tomb {
    #[wasm_bindgen(constructor)]
    pub async fn new(
        endpoint: String,
        token: String,
    ) -> Result<Tomb, JsValue> {
        log!("tomb-wasm: new()");
        utils::set_panic_hook();
        let metadata_service = MetadataService::new(endpoint, token);
        let buckets = metadata_service.read_buckets().await.unwrap();
        Ok(Tomb { 
            buckets, 
            metadata_service,
            blockstore: MemoryBlockStore::default(),
            private_forest: None,
            private_directory: None
        })
    }
    
    /// Return a list of id:name pairs for all buckets accessible to the user
    /// # Returns
    /// * Vec<(String, String)> - A vector of id:name pairs
    pub fn buckets(&self) -> Result<JsValue, JsValue> {
        log!("tomb-wasm: bucket()");
        serde_wasm_bindgen::to_value(&self.buckets)
            .map_err(|_| JsValue::from_str("Error serializing buckets"))
    }

    /// Load a specific bucket's metadata into memory, by name
    /// # Arguments
    /// * `name` - The name of the bucket to load
    /// * `key` - The key to decrypt the bucket's metadata
    #[wasm_bindgen(js_name = loadBucket)]
    pub async fn load_bucket(&mut self, _name: String, key: JsValue) -> Result<(), JsValue> {
        log!("tomb-wasm: load_bucket()");
        // Import the key from a pkcs8 string
        let rsa_private_key: RsaPrivateKey = utils::string_to_rsa_key(key)?;
        // TODO: Remove this at some point
        let rsa_public_key: RsaPublicKey = rsa_private_key.get_public_key();

        // TODO: Use real bucket_id and fingerprint, and real data from the metadata service
        // Read the encrypted share key from the metadata service and decrypt it
        let _enc_share_key_vec = self.metadata_service.read_enc_share_key("bucket_id".to_string(), "fingerprint".to_string()).await.unwrap();
         // TODO: Remove this at some point -- need it so long as we're using fake data, since this will fail if the key was not actually encrypted with the public key
        let enc_share_key_vec = rsa_public_key.encrypt(&_enc_share_key_vec).await.unwrap();
        let share_key_vec = rsa_private_key.decrypt(&enc_share_key_vec).await.unwrap();        
        // Just make sure the share key is what we originally encrypted
        assert_eq!(share_key_vec, _enc_share_key_vec);

        // Convert the share key to a TemporalKey
        let key_bytes = utils::expect_bytes::<KEY_BYTE_SIZE>(share_key_vec)?;
        let _temporal_key = TemporalKey::from(key_bytes);
        
        // TODO: Load metadata CAR and write to self.blockstore
        let _metadata_vec = self.metadata_service.read_metadata("bucket_id".to_string()).await.unwrap();
        // TODO: Read the bytes into a blockstore

        // TODO: Load the private directory and private forest using the decrypted share key 
        // For now just test import a private directory and private forest
        let rng = &mut thread_rng();
        self.private_forest = Some(PrivateForest::new());
        self.private_directory = Some(PrivateDirectory::new(
            Namefilter::new(),
            Utc::now(),
            rng
        ));

        match Some(self.private_directory.as_ref().unwrap()) {
            Some(private_directory) => {
                // let private_directory: &mut Rc<PrivateDirectory> = &mut private_directory.clone();
                let private_directory: &mut Rc<PrivateDirectory> = &mut Rc::new(private_directory.clone());
                private_directory
                    .mkdir(
                        &["test".to_string(), "path".to_string()],
                        true,
                        Utc::now(),
                        self.private_forest.as_ref().unwrap(),
                        &self.blockstore,
                        rng
                    ).await.unwrap();
            },
            None => {
                log!("tomb-wasm: load_bucket() - private_directory is None");
            }
        }
        Ok(())
    }

    /* Private Direcotry Methods */

    // /// List the entries of the current private directory at a given path
    // /// # Arguments
    // /// * `path_segments` - A vector of path segments to list
    // /// # Returns
    // /// * TODO: decide on return type
    // pub fn ls(&self, path_segments: &Array) -> JsResult<Promise> {
    //     log!("tomb-wasm: ls()");
    //     let private_directory = self.private_directory_rc();
    //     let private_forest = self.private_forest_rc();
    //     let path_segments = utils::convert_path_segments(path_segments)?;
    //     Ok(
    //         future_to_promise(async move {
    //             let result = private_directory.ls(
    //                 &path_segments,
    //                 true, // Search latest -- TODO: What is this for?
    //                 private_forest.as_ref(),
    //                 &self.blockstore
    //             ).await;

    //             let result = result
    //             .iter()
    //             .flat_map(|(name, metadata)| utils::create_ls_entry(name, metadata))
    //             .collect::<Array>();

    //             Ok(result)
    //         })
    //     )
    // }
}
// Private methods, not exported to JavaScript.
impl Tomb {
    pub fn private_forest_rc(&self) -> Rc<PrivateForest> {
        Rc::new(self.private_forest.as_ref().unwrap().clone())
    }

    pub fn private_directory_rc(&self) -> Rc<PrivateDirectory> {
        Rc::new(self.private_directory.as_ref().unwrap().clone())
    }
}




