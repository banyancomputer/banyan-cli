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
use js_sys::{Array, Object, Reflect};
use std::convert::TryFrom;
pub use web_sys::CryptoKey;

use wasm_bindgen::prelude::*;

use crate::utils::{set_panic_hook, JsResult};

// #[derive(Debug)]
#[wasm_bindgen]
pub struct TombWasm {
    /// Client for interacting with the Banyan API
    banyan_client: BanyanClient,

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
    pub fn new(web_api_key: CryptoKey, account_id: String, api_endpoint: String) -> Self {
        set_panic_hook();
        log!("tomb-wasm: new()");
        let banyan_client = BanyanClient::new(api_endpoint, account_id, web_api_key);
        Self {
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
        log!("tomb-wasm: get_total_storage()");
        let client = &self.banyan_client;
        let total_storage = client.get_total_storage().await?;
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
        log!("tomb-wasm: get_trash_bucket()");
        let client = &self.banyan_client;
        let bucket = client.get_trash_bucket().await?;
        let bucket = JsValue::try_from(bucket.clone()).unwrap();
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
        log!("tomb-wasm: get_buckets()");
        let client = &self.banyan_client;
        let buckets = client.get_buckets().await?;
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
        log!("tomb-wasm: get_snapshots()");
        let client = &self.banyan_client;
        let snapshots = client.get_snapshots().await?;
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
    pub async fn get_bucket_storage(&self, _bucket_id: &str) -> JsResult<u64> {
        log!("tomb-wasm: get_bucket_storage()");
        let client = &self.banyan_client;
        let storage = client.get_bucket_storage(_bucket_id).await?;
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
    pub async fn get_bucket_keys(&self, _bucket_id: &str) -> JsResult<Array> {
        log!("tomb-wasm: get_bucket_keys()");
        let client = &self.banyan_client;
        let keys = client.get_bucket_keys(_bucket_id).await?;
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
    pub async fn get_bucket_snapshots(&self, _bucket_id: &str) -> JsResult<Array> {
        log!("tomb-wasm: get_bucket_snapshots()");
        // Call the api
        let client = &self.banyan_client;
        let snapshots = client.get_bucket_snapshots(_bucket_id).await?;
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
    #[wasm_bindgen(js_name = load)]
    pub async fn load(&mut self, _bucket_id: &str) -> JsResult<()> {
        log!("tomb-wasm: load_bucket({})", _bucket_id);
        let banyan_client = &self.banyan_client;
        let bucket = banyan_client.load_bucket(_bucket_id).await?;
        self.buckets.insert(_bucket_id.to_string(), bucket); // Release the lock
        Ok(())
    }

    /// Sync a bucket with the remote
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to sync
    #[wasm_bindgen(js_name = syncBucket)]
    pub async fn sync_bucket(&mut self, _bucket_id: &str) -> JsResult<()> {
        log!("tomb-wasm: sync_bucket({})", _bucket_id);
        // Get the bucket
        let bucket = match self.buckets.get_mut(_bucket_id) {
            Some(bucket) => bucket,
            None => {
                panic!("Bucket not loaded");
            }
        };
        // Call the api
        let client = &self.banyan_client;
        client.sync_bucket(&bucket).await?;
        Ok(())
    }

    /// Delete a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to delete
    // TODO: what is the return type?
    #[wasm_bindgen(js_name = deleteBucket)]
    pub async fn delete_bucket(&self, _bucket_id: String) -> JsResult<()> {
        log!("tomb-wasm: delete_bucket()");
        self.banyan_client.delete_bucket(&_bucket_id).await?;
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
        log!("tomb-wasm: request_bucket_access({})", _bucket_id);
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
        log!("tomb-wasm: approve_bucket_access({})", _bucket_key_id);
        let bucket_id = "1".to_string();
        let bucket = match self.buckets.get(&bucket_id) {
            Some(bucket) => bucket,
            None => {
                panic!("Bucket not loaded");
            }
        };
        if bucket.is_locked() {
            panic!("Bucket is locked");
        };
        Ok(())
    }

    /* Bucket Interface -- once a bucket is loaded, we can interact with it by its id */

    /// Unlock a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to unlock
    /// * `wrapping_key` - The wrapping key to unlock the bucket with
    #[wasm_bindgen(js_name = unlock)]
    pub async fn unlock(
        &mut self,
        bucket_id: &str,
        _wrapping_key: CryptoKey,
    ) -> JsResult<()> {
        log!("tomb-wasm: unlock({})", bucket_id);
        let bucket = match self.buckets.get_mut(bucket_id) {
            Some(bucket) => bucket,
            None => {
                panic!("Bucket not loaded");
            }
        };
        if !bucket.is_locked() {
            panic!("Bucket is already unlocked");
        };
        // TODO: Implement with wrapping key
        bucket.unlock().await?;
        Ok(())
    }

    /// List the contents of a bucket at a path
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to list contents for
    /// * `path` - The path to list contents for
    /// * `version` - The version to list contents for (optional)
    /// # Returns
    /// An array of entries TODO: What form is this?
    #[wasm_bindgen(js_name = ls)]
    pub async fn ls(&self, bucket_id: &str, path: &str, version: Option<String>) -> JsResult<Array> {
        log!("tomb-wasm: ls({}/{})", bucket_id, path);
        let bucket = match self.buckets.get(bucket_id) {
            Some(bucket) => bucket,
            None => {
                panic!("Bucket not loaded")
            }
        };
        if bucket.is_locked() {
            panic!("Bucket is locked");
        };
        let path_segments = path.split('/').collect::<Vec<&str>>();
        let entries = bucket.ls(path_segments).await?;
        let entries = entries
            .iter()
            .map(|(name, entry)| {
                let obj = Object::new();
                Reflect::set(&obj, &"name".into(), &name.into()).unwrap();
                Reflect::set(
                    &obj,
                    &"metadata".into(),
                    &JsValue::try_from(entry.clone()).unwrap(),
                )
                .unwrap();
                obj
            })
            .collect::<Array>();
        Ok(entries)
    }

    /// Snapshot a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to snapshot
    /// # Returns
    /// TODO: What form is this?
    #[wasm_bindgen(js_name = snapshot)]
    pub async fn snapshot(&mut self, bucket_id: &str) -> JsResult<()> {
        log!("tomb-wasm: snapshot({})", bucket_id);
        // Get the bucket
        let bucket = match self.buckets.get_mut(bucket_id) {
            Some(bucket) => bucket,
            None => {
                panic!("Bucket not loaded");
            }
        };
        // Check if the bucket is unlocked
        if bucket.is_locked() {
            panic!("Bucket is locked");
        };
        // Call the bucket
        bucket.snapshot().await?;
        // Ok
        Ok(())
    }

    /// Read a file from a bucket
    ///     Read / Download a File (takes a path to a file inside the bucket, not available for cold only buckets)
    ///     Allows reading at a version
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to read from
    /// * `path` - The path to read from
    /// * `version` - The version to read from (optional)
    /// # Returns
    /// TODO: What form is this?
    /// TODO: Acutal implementation
    #[wasm_bindgen(js_name = read)]
    pub async fn read(
        &self,
        bucket_id: &str,
        _path: &str,
        _version: Option<String>,
    ) -> JsResult<()> {
        log!("tomb-wasm: read({}/{})", bucket_id, _path);
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
        // Ok
        Ok(())
    }

    /// Delete a file from a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to delete from
    /// * `path` - The path to delete from
    /// # Returns
    /// TODO: What form is this?
    #[wasm_bindgen(js_name = delete)]
    pub async fn delete(&self, bucket_id: &str, _path: &str) -> JsResult<()> {
        log!("tomb-wasm: delete({}/{})", bucket_id, _path);
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
        // Ok
        Ok(())
    }

    /// Get a file's / folder's metadata from a bucket
    ///     Get file / folder versions (takes a path to a file or directory inside the bucket)
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to download from
    /// * `path` - The path to download from
    /// # Returns
    /// TODO: What form is this?
    /// For now we'll just return some sample data
    /// ```json
    /// {
    ///     "id": "uuid",
    ///     "bucket_id": "uuid",
    ///     "path": "string",
    ///     "type": "string",
    ///     "cid": "string",
    ///     "size": "u64",
    ///     "versions": "array",
    ///     "created_at": "string",
    ///     "updated_at": "string",
    /// }
    #[wasm_bindgen(js_name = getMetadata)]
    pub async fn get_metadata(&self, bucket_id: &str, path: &str) -> JsResult<JsValue> {
        log!("tomb-wasm: get_metadata({}/{})", bucket_id, path);
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
        // Return some sample data
        let res = JsValue::from_serde(&serde_json::json!({
            "id": "uuid",
            "bucket_id": bucket_id,
            "path": path,
            "type": "file",
            "cid": "Qmabc",
            "size": 1024,
            "versions": [
                "1",
                "2",
                "3"
            ],
            "created_at": "today",
            "updated_at": "today",
        }))
        .unwrap();
        Ok(res)
    }

    /// Create a directory in a bucket
    /// Create directory (takes a path to a non-existent directory)
    /// Will create parent directories as need to create the file directory
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to create a directory in
    /// * `path` - The path to create a directory in
    /// # Returns
    /// TODO: What form is this?
    #[wasm_bindgen(js_name = createDirectory)]
    pub async fn create_directory(&self, bucket_id: &str, _path: &str) -> JsResult<()> {
        log!("tomb-wasm: create_directory({}/{})", bucket_id, _path);
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
        // Ok
        Ok(())
    }

    /// Rename a file or directory in a bucket
    ///     Rename (tasks a source and destination path, destination must not exist)
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to rename in
    /// * `source` - The source path to rename
    /// * `destination` - The destination path to rename to
    /// # Returns
    /// TODO: What form is this?
    #[wasm_bindgen(js_name = rename)]
    pub async fn rename(&self, bucket_id: &str, _source: &str, _destination: &str) -> JsResult<()> {
        log!("tomb-wasm: rename({}/{}/{})", bucket_id, _source, _destination);
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
        // Ok
        Ok(())
    }

    /// Migrate a file or directory in a bucket to another bucket
    ///     Migrate (takes a list of files and directories in the current bucket, another OpenedBucket, and a destination in the OpenedBucket to move the data into)
    /// # Arguments
    /// * `source_bucket_id` - The id of the bucket to migrate from
    /// * `destination_bucket_id` - The id of the bucket to migrate to
    /// * `sources` - The source path to migrate
    /// * `destinations` - The destination path to migrate to
    /// # Returns
    /// TODO: What form is this?
    #[wasm_bindgen(js_name = migrate)]
    pub async fn migrate(
        &self,
        _source_bucket_id: &str,
        _destination_bucket_id: &str,
        _sources: Array,
        _destinations: Array,
    ) -> JsResult<()> {
        log!("tomb-wasm: migrate({}/{})", _source_bucket_id, _destination_bucket_id);
        // Get the bucket
        let source = match self.buckets.get(_source_bucket_id) {
            Some(bucket) => bucket,
            None => {
                panic!("Bucket not loaded");
            }
        };
        // Check if the bucket is unlocked
        if source.is_locked() {
            panic!("Bucket is locked");
        };

        // Get the bucket
        let destination = match self.buckets.get(_destination_bucket_id) {
            Some(bucket) => bucket,
            None => {
                panic!("Bucket not loaded");
            }
        };

        // Check if the bucket is unlocked
        if destination.is_locked() {
            panic!("Bucket is locked");
        };

        // Ok
        Ok(())
    }

    /// Upload a file to a bucket
    /// # Arguments
    /// * `bucket_id` - The id of the bucket to upload to
    /// * `path` - The path to upload to
    /// * `file` - The file to upload
    /// # Returns
    /// TODO: What form is this?
    ///  Upload file (takes a path to a non-existent file, and a ReadableStream)
    ///    Should produce a promise for a completed upload and a way to track its progress
    ///    I suspect this is going to be the hardest to implement, I'd save it for last
    #[wasm_bindgen(js_name = upload)]
    pub async fn upload(
        &self,
        bucket_id: &str,
        _path: &str,
        _file: JsValue,
    ) -> JsResult<()> {
        log!("tomb-wasm: upload({}/{})", bucket_id, _path);
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
        // Ok
        Ok(())
    }

    // Snapshot Management

    /// Purge a snapshot
    /// # Arguments
    /// * `snapshot_id` - The id of the snapshot to purge
    /// # Returns
    /// TODO: What form is this?
    #[wasm_bindgen(js_name = purgeSnapshot)]
    pub async fn purge_snapshot(&self, _snapshot_id: &str) -> JsResult<()> {
        // Call the api
        let client = &self.banyan_client;
        client.purge_snapshot(_snapshot_id).await?;
        // Ok
        Ok(())
    }
}
