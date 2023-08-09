//! This crate contains modules which are compiled to WASM
#![warn(rust_2018_idioms)]
/// Banyan API
mod banyan;
/// Expose blockstore functionality
mod blockstore;
/// Expose Errors
mod error;
/// Fetch remote data
mod fetch;
/// Misc utilities
mod utils;

use std::collections::HashMap;

use banyan::bucket::Bucket as BanyanBucket;
use banyan::client::Client as BanyanClient;
use banyan::snapshot::Snapshot as BanyanSnapshot;
use gloo::console::log;
use js_sys::{Array, Reflect, Object};
use std::convert::TryFrom;
use tomb_crypt::prelude::*;
pub use web_sys::CryptoKey;

use wasm_bindgen::prelude::*;

use crate::utils::{JsResult, set_panic_hook};

// #[derive(Debug)]
#[wasm_bindgen]
pub struct TombWasm {
    /// Client for interacting with the Banyan API
    banyan_client: BanyanClient,

    // Needed for interacting with Buckets
    /// Wrapping key for unlocking buckets
    wrapping_key: EcEncryptionKey,
    /// Map of bucket IDs to buckets
    buckets: HashMap<String, BanyanBucket>,
    /// Map of snapshot IDs to snapshots
    snapshots: HashMap<String, BanyanSnapshot>,
}

#[wasm_bindgen]
/// TombWasm exposes the functionality of Tomb in a WASM module
impl TombWasm {
    #[wasm_bindgen(constructor)]
    /// Create a new TombWasm instance
    pub fn new(
        web_wrapping_key: CryptoKey,
        _web_api_key: CryptoKey,
        account_id: String,
        api_endpoint: String,
    ) -> Self {
        set_panic_hook();
        // Convert the wrapping key
        let wrapping_key = EcEncryptionKey::from(web_wrapping_key);
        // Convert the api key
        // let api_key = EcSignatureKey::from(web_api_key);
        // Create a new api
        let banyan_client = BanyanClient::new(api_endpoint, account_id, _web_api_key);
        // Ok
        Self {
            wrapping_key,
            banyan_client,
            buckets: HashMap::new(),
            snapshots: HashMap::new(),
        }
    }

    /*
     * API Interface
     */

    // Account Metadata

    /// Get the Total Storage for the current account
    /// # Returns
    /// The total storage used by the account, in bytes
    #[wasm_bindgen(js_name = getTotalStorage)]
    pub async fn get_total_storage(&self) -> JsResult<u64> {
        // Call the api
        let total_storage = self.banyan_client.get_total_storage().await?;
        // Ok
        Ok(total_storage)
    }

    /// Get Metadata for the Trash Bucket
    /// # Returns
    /// The metadata for the trash bucket of the form:
    /// ```json
    /// {
    ///  "id": "uuid",
    /// "bucket_type": "uuid",
    /// "name": "string"
    /// }
    /// ```
    #[wasm_bindgen(js_name = getTrashBucket)]
    pub async fn get_trash_bucket(&self) -> JsResult<JsValue> {
        // Call the api
        let bucket = self.banyan_client.get_trash_bucket().await?;
        // Convert the bucket
        let bucket = JsValue::try_from(bucket.clone()).unwrap();
        // Ok
        Ok(bucket)
    }

    /// List the buckets for the current account
    /// # Returns
    /// An array of buckets of the form:
    /// ```json
    /// [
    ///    {
    ///       "id": "uuid",
    ///      "bucket_type": "uuid",
    ///     "name": "string"
    ///   }
    /// ]
    /// ```
    #[wasm_bindgen(js_name = getBuckets)]
    pub async fn get_buckets(&self) -> JsResult<Array> {
        // Get the buckets
        let buckets = self.banyan_client.get_buckets().await?;
        let buckets = buckets
            .iter()
            .map(|bucket| {
                let value = JsValue::try_from(bucket.clone()).unwrap();
                value
            })
            .collect::<Array>();
        // Ok
        Ok(buckets)
    }

    /// List the snapshots for the current account
    /// # Returns
    /// An array of snapshots of the form:
    /// ```json
    ///  [
    ///     {
    ///         "id": "uuid",
    ///        "bucket_id": "uuid",
    ///      "snapshot_type": "string",
    ///   "version": "string"
    ///     }
    /// ]
    #[wasm_bindgen(js_name = getSnapshots)]
    pub async fn get_snapshots(&self) -> JsResult<Array> {
        // Get the snapshots
        let snapshots = self.banyan_client.get_snapshots().await?;
        let snapshots = snapshots
            .iter()
            .map(|snapshot| {
                let value = JsValue::try_from(snapshot.clone()).unwrap();
                value
            })
            .collect::<Array>();
        // Ok
        Ok(snapshots)
    }

    // Bucket Metadata

    /// Get the total storage used by a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to get storage for
    #[wasm_bindgen(js_name = getBucketStorage)]
    pub async fn get_bucket_storage(&self, _bucket_id: String) -> JsResult<u64> {
        // Call the api
        let storage = self.banyan_client.get_bucket_storage(&_bucket_id).await?;
        // Ok
        Ok(storage)
    }

    /// List bucket keys for a bucket
    /// Returns an array of public keys in the form:
    /// ```json
    /// [
    ///   {
    ///    "id": "uuid",
    ///    "bucket_id": "uuid",
    ///    "pem": "string"
    ///    "approved": "bool"
    ///  }
    /// ]
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to list keys for
    // TODO: Replace with API call
    #[wasm_bindgen(js_name = getBucketKeys)]
    pub async fn get_bucket_keys(&self, _bucket_id: String) -> JsResult<Array> {
        // Call the api
        let keys = self.banyan_client.get_bucket_keys(&_bucket_id).await?;
        // Convert the keys
        let keys = keys
            .iter()
            .map(|key| {
                let value = JsValue::try_from(key.clone()).unwrap();
                value
            })
            .collect::<Array>();
        // Ok
        Ok(keys)
    }

    /// List snapshots for a bucket
    /// Returns an array of snapshots in the form:
    /// ```json
    /// [
    ///  {
    ///   "id": "uuid",
    ///  "bucket_id": "uuid",
    /// "snapshot_type": "string",
    /// "version": "string"
    /// }
    /// ]
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to list snapshots for
    #[wasm_bindgen(js_name = getBucketSnapshots)]
    pub async fn get_bucket_snapshots(&self, _bucket_id: String) -> JsResult<Array> {
        // Call the api
        let snapshots = self.banyan_client.get_bucket_snapshots(&_bucket_id).await?;
        // Convert the snapshots
        let snapshots = snapshots
            .iter()
            .map(|snapshot| {
                let value = JsValue::try_from(snapshot.clone()).unwrap();
                value
            })
            .collect::<Array>();
        // Ok
        Ok(snapshots)
    }

    // Bucket Management

    /// Initialize a bucket by id. Associates buckets within TombWasm Client
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to load
    #[wasm_bindgen(js_name = loadBucket)]
    pub async fn load_bucket(&mut self, _bucket_id: &str) -> JsResult<()> {
        log!("tomb-wasm: load_bucket()");
        let banyan_client = &self.banyan_client;
        // Call the api
        log!("tomb-wasm: load_bucket() - calling api");
        let bucket = banyan_client.load_bucket(_bucket_id).await?;
        // Add the bucket to the map
        log!("tomb-wasm: load_bucket() - bucket loaded");
        self.buckets.insert(_bucket_id.to_string(), bucket);
        // log!("tomb-wasm: load_bucket() - bucket inserted");
        Ok(())
    }

    /// Sync a bucket with the remote
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to sync
    #[wasm_bindgen(js_name = syncBucket)]
    pub async fn sync_bucket(&self, _bucket_id: &str) -> JsResult<()> {
        let bucket = self.buckets.get(_bucket_id).unwrap();
        // Call the api
        self.banyan_client.sync_bucket(&bucket).await?;
        // Ok
        Ok(())
    }

    /// Delete a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to delete
    // TODO: what is the return type?
    #[wasm_bindgen(js_name = deleteBucket)]
    pub async fn delete_bucket(&self, _bucket_id: String) -> JsResult<()> {
        // Call the api
        self.banyan_client.delete_bucket(&_bucket_id).await?;
        // Ok
        Ok(())
    }

    /// Request access to a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to request access to
    /// * `public_key` - The public key to approve access for
    // TODO: what is the return type?
    #[wasm_bindgen(js_name = requestBucketAccess)]
    pub async fn request_bucket_access(
        &self,
        _bucket_id: String,
        _public_key: CryptoKey,
    ) -> JsResult<()> {
        // NOTE: leave commented out until we have a way to convert the public key
        // Convert the public key
        // let public_key = EcPublicEncryptionKey::from(_public_key);
        // Call the api
        // self.banyan_client
        //     .request_bucket_access(&_bucket_id, &public_key)
        //     .await?;
        // Ok
        Ok(())
    }

    /// Approve bucket Access (take a UUID of a specific key request)
    /// Internally this is going to request the public key request, and encrypt the WNFS key with the associated public key, update the metadata and perform a sync
    /// # Arguments
    /// * `bucket_key_id` - The id of the key request to approve
    #[wasm_bindgen(js_name = approveBucketAccess)]
    pub async fn approve_bucket_access(&self, _bucket_key_id: String) -> JsResult<()> {
        // Get the pem
        // Get the bucket id from the bucket_key
        let bucket_id = "1".to_string();

        // Check if the bucket is loaded and unlocked
        let bucket = match self.buckets.get(&bucket_id) {
            Some(bucket) => bucket,
            None => {
                panic!("Bucket not loaded");
            }
        };
        // Check if the bucket is unlocked
        if bucket.is_locked() {
            panic!("Bucket is locked");
        };
        // Unlock the bucket
        // Share the bucket with the public key
        // Sync the bucket
        Ok(())
    }

    /* Bucket Interface -- once a bucket is loaded, we can interact with it by its id */

    /// Unlock a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to unlock
    /// * `wrapping_key` - The wrapping key to unlock the bucket with
    #[wasm_bindgen(js_name = unlockBucket)]
    pub async fn unlock_bucket(
        &mut self,
        bucket_id: &str,
        _wrapping_key: CryptoKey,
    ) -> JsResult<()> {
        // Get the bucket
         // Check if the bucket is loaded and unlocked
         let bucket = match self.buckets.get_mut(bucket_id) {
            Some(bucket) => bucket,
            None => {
                panic!("Bucket not loaded");
            }
        };
        // Check if the bucket is unlocked
        if !bucket.is_locked() {
            panic!("Bucket is already unlocked");
        };

        // Convert the wrapping key
        // let wrapping_key = EcEncryptionKey::from(_wrapping_key);
        // Unlock the bucket
        bucket.unlock().await?;
        // Ok
        Ok(())
    }

    /// List the contents of a bucket at a path
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to list contents for
    /// * `path` - The path to list contents for
    /// # Returns
    /// An array of entries TODO: What form is this?
    #[wasm_bindgen(js_name = lsBucket)]
    pub async fn ls_bucket(&self, bucket_id: &str, path: &str) -> JsResult<Array> {
        // Get the bucket
        let bucket = match self.buckets.get(bucket_id) {
            Some(bucket) => bucket,
            None => {
                panic!("Bucket not loaded");
            }
        };
        // Check if the bucket is unlocked
        if bucket.is_locked() {
            panic!("Bucket is locked");
        };
        // Break the path into segments
        let path_segments = path.split('/').collect::<Vec<&str>>();
        // Call the bucket
        let entries = bucket.ls(path_segments).await?;
        // Map the entries to JsValues
        let entries = entries
            .iter()
            .map(|(name, entry)| {
                let obj = Object::new();
                Reflect::set(&obj, &"name".into(), &name.into()).unwrap();
                Reflect::set(&obj, &"metadata".into(), &JsValue::try_from(entry.clone()).unwrap()).unwrap();
                obj
            })
            .collect::<Array>();
        // Ok
        Ok(entries)
    }

    /*

    Unlock (takes a private ECDH key and attempts to open the bucket), returns an OpenedBucket


    List current bucket keys

    Retrieve current storage use by bucket

    List contents (takes a path to a directory inside the bucket)

    Read / Download a File (takes a path to a file inside the bucket, not available for cold only buckets)

    Get file / folder versions (takes a path to a file or directory inside the bucket)

    Download file version (takes a path to a file, and a specific version identifier)

    View folder version (takes a path to folder, and a specific version identifier)

    Create directory (takes a path to a non-existent directory)
        Will create parent directories as need to create the file directory

    Rename (tasks a source and destination path, destination must not exist)

    Migrate (takes a list of files and directories in the current bucket, another OpenedBucket, and a destination in the OpenedBucket to move the data into)

    Delete a File (tasks a path)

    Sync
        Takes all outstanding changes to file/directories, and publishes them to our platform

    Snapshot (takes no parameters)

    List snapshots (takes no parameters, returns list of Snapshots)

    Upload file (takes a path to a non-existent file, and a ReadableStream)

    Should produce a promise for a completed upload and a way to track its progress

    I suspect this is going to be the hardest to implement, I'd save it for last
    */
    /*  Snapshot

        Get details about specific snapshot

        Restore to bucket (takes a specific bucket ID)

        Purge keys (takes a signed authorization by an approved key)
    */
}
