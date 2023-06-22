mod metadata;
mod utils;

use metadata::types::Bucket;
use wasm_bindgen::prelude::*;
use web_sys::{ 
    console,
    CryptoKey,
};
use wnfs::{
    common::MemoryBlockStore,
    private::PrivateDirectory
};

// Our optional WeeAlloc allocator
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// Our logging macro
macro_rules! log {
    ( $( $t:tt )* ) => {
        console::log_1(&format!( $( $t )* ).into());
    }
}

#[allow(dead_code)]
#[wasm_bindgen]
pub struct Tomb {
    buckets: Vec<Bucket>,
    metadata_service: metadata::types::Service,
    blockstore: MemoryBlockStore,
    private_directory: Option<PrivateDirectory>,
}

// Public methods, exported to JavaScript.
#[wasm_bindgen]
impl Tomb {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // log!("tomb-wasm: new()");
        utils::set_panic_hook();
        Tomb { 
            buckets: Vec::new(), 
            metadata_service: metadata::types::Service::new("".to_string(), "".to_string()), 
            blockstore: MemoryBlockStore::new(), 
            private_directory: None
        }
    }

    /// Initializes the Tomb instance with initial metadata
    pub fn init(&mut self) -> Result<(), JsValue> {
        // log!("tomb-wasm: init()");
        // TODO: Read buckets from metadata service 
        self.buckets = [Bucket {
            id: "id".to_string(),
            name: "name".to_string(),
            owner: "owner".to_string(),
            entrypoint: "entrypoint".to_string(),
        }].to_vec();
        Ok(())
    }

    /// Load a specific bucket's metadata into memory, by name
    /// # Arguments
    /// * `name` - The name of the bucket to load
    /// * `key` - The key to decrypt the bucket's metadata
    pub fn load_bucket(&mut self, _name: String, _key: CryptoKey) -> Result<(), JsValue> {
        // log!("tomb-wasm: load_bucket()");
        // TODO: Load metadata CAR and write to self.blockstore
        // TODO: Load the encrypted share key
        // TODO: Decrypt the share key
        // TODO: Load the private directory using self.blockstore and the decrypted share key
        unimplemented!()
    }

    /// List the entries of the current private directory at a given path
    /// # Arguments
    /// * `path` - The path to list
    /// # Returns
    /// * TODO: decide on return type
    pub fn list(&self, _path: String) -> Result<(), JsValue> {
        // log!("tomb-wasm: list()");
        unimplemented!()
    }
}

// Private methods, not exported to JavaScript.
impl Tomb {
    // pub fn example_method(&mut self) {
    //     log!("This is not exported to JavaScript.");
    // }
}




