use std::rc::Rc;
use std::convert::TryFrom;
use chrono::Utc;
use gloo::console::log;
use js_sys::{Array, Reflect, Object};
use wnfs::common::Metadata as FsEntryMetadata;
use futures_util::StreamExt;
use tomb_common::blockstore::TombBlockStore;
use tomb_crypt::prelude::*;
use tomb_common::banyan::client::Client;
use tomb_common::banyan::models::{
    bucket::Bucket, 
    metadata::Metadata
};
use tomb_common::blockstore::carv2_memory::CarV2MemoryBlockStore as BlockStore;
use tomb_common::keys::manager::Manager;
use wasm_bindgen::prelude::*;
use wnfs::private::{PrivateForest, PrivateDirectory};
use web_sys::CryptoKey;

use crate::error::TombWasmError;
use crate::utils::JsResult;
use crate::types::{WasmBucket, WasmBucketEntry};

/// Mount point for a Bucket in WASM
/// Enables to call Fs methods on a Bucket, pulling metadata from a remote
#[wasm_bindgen]
pub struct WasmMount {
    /* Remote client */
    client: Client,

    /* Remote metadata */
    /// Bucket Metadata
    bucket: Bucket,
    /// Currently initialized version of Fs Metadata
    metadata: Metadata,
    /// Whether or not the bucket is locked
    locked: bool,

    /* Fs Exposure  */

    /// Encrypted metadata within a local memory blockstore
    fs_metadata: BlockStore,

    /// Private Forest over Fs Metadata
    fs_metadata_forest: Option<Rc<PrivateForest>>,
    /// Private Forest over Fs Content
    fs_content_forest: Option<Rc<PrivateForest>>,
    /// Reference to the root directory of the Fs
    fs_dir: Option<Rc<PrivateDirectory>>,

    /// Key manager on top of Fs Metadata
    fs_key_manager: Option<Manager>,
}

impl WasmMount {
    pub async fn new(bucket: WasmBucket, key: &EcEncryptionKey, client: &mut Client) -> Result<Self, TombWasmError> {
        let bucket = Bucket::from(bucket);
        let fs_metadata = BlockStore::new().expect("could not create blockstore");
        let (fs_metadata_forest, fs_content_forest, fs_dir, fs_key_manager, _) =
            fs_metadata.init(key).await.expect("could not init blockstore");
        let (metadata, _) = Metadata::push(
            bucket.id,
            fs_metadata.get_root().expect("could not get root").to_string(),
            "fake-metadata-cid".to_string(),
            0,
            fs_metadata.get_data(),
            client
        ).await.expect("could not push metadata");
        // Ok
        Ok(Self {
            client: client.to_owned(),
            bucket,
            metadata,
            locked: false,
            fs_metadata,

            fs_metadata_forest: Some(fs_metadata_forest),
            fs_content_forest: Some(fs_content_forest),
            fs_dir: Some(fs_dir),
            fs_key_manager: Some(fs_key_manager)
        })
    }
    /// Initialize a new Wasm callable mount with metadata for a bucket and a client
    pub async fn pull(bucket: WasmBucket, client: &mut Client) -> Result<Self, TombWasmError> {
        // Get the underlying bucket
        let bucket = Bucket::from(bucket);
        // Get the metadata associated with the bucket
        let metadata = Metadata::read_current(bucket.id, client)
            .await
            .map_err(|_| TombWasmError::unknown_error())?;
        // Pull the Fs metadata on the matching entry
        let mut stream = metadata.pull(client).await.expect("could not pull metedata");
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }
        let fs_metadata = BlockStore::try_from(data).expect("could not create metadata as blockstore");

        // Ok
        Ok(Self {
            client: client.to_owned(),
            bucket,
            metadata: metadata.to_owned(),
            locked: true,
            fs_metadata,

            fs_metadata_forest: None,
            fs_content_forest: None,
            fs_dir: None,
            fs_key_manager: None
        })
    }

    /// Unlock the current fs_metadata
    pub async fn unlock(mut self, key: &EcEncryptionKey) -> Result<Self, TombWasmError> {
        log!("tomb-wasm: mount/unlock()");
        // Unlock the components over the FS
        let (fs_metadata_forest, fs_content_forest, fs_dir, fs_key_manager, _) = 
            self.fs_metadata.unlock(&key).await.expect("could not unlock fs");
        self.fs_metadata_forest = Some(fs_metadata_forest);
        self.fs_content_forest = Some(fs_content_forest);
        self.fs_dir = Some(fs_dir);
        self.fs_key_manager = Some(fs_key_manager);
        self.locked = false;
        // Ok
        Ok(self)
    }

    fn get_dir(&self) -> &Rc<PrivateDirectory> {
        self.fs_dir
            .as_ref()
            .unwrap_or_else(|| panic!("Bucket is locked"))
    }
    fn get_metadata_forest(&self) -> &Rc<PrivateForest> {
        self.fs_metadata_forest.as_ref().unwrap()
    }
    fn get_manager(&self) -> &Manager {
        self.fs_key_manager.as_ref().unwrap()
    }
}

#[wasm_bindgen]
impl WasmMount {
    pub fn is_locked(&self) -> bool {
        self.locked == true
    }
    
    // Sync the mount
    pub async fn sync(&mut self) -> JsResult<()> {
        log!("tomb-wasm: sync()");
        panic!("not implemented")
    }


    pub async fn ls(
        &self,
        path: String
    ) -> JsResult<Array> {
        log!("tomb-wasm: ls({})", &path);
        // let path_segments = path.split('/').collect::<Vec<String>>();
        let path_segments = path.split('/').collect::<Vec<&str>>();
        let path_segments = path_segments.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        let dir = self.get_dir();
        let metadata_forest = self.get_metadata_forest();
        let entries = dir
            .ls(
                path_segments.as_slice(),
                true,
                metadata_forest,
                &self.fs_metadata,
            )
            .await
            .map_err(|_| TombWasmError::unknown_error())?;
        let entries = entries
            .iter()
            .map(|(name, entry)| {
                let obj = Object::new();
                Reflect::set(&obj, &"name".into(), &name.into()).unwrap();
                Reflect::set(
                    &obj,
                    &"metadata".into(),
                    &WasmBucketEntry(entry.clone()).try_into().expect("could not convert entry"),
                )
                .unwrap();
                obj
            })
            .collect::<Array>();
        Ok(entries)
        // Return some sample data
        // file size
        // file cid or id
        // let vec = [
        //     (
        //         "puppy.png".to_string(),
        //         WasmBucketEntry(FsEntryMetadata::new(Utc::now())),
        //     ),
        //     (
        //         "chonker.jpg".to_string(),
        //         WasmBucketEntry(FsEntryMetadata::new(Utc::now())),
        //     ),
        //     (
        //         "floof_doof.mp3".to_string(),
        //         WasmBucketEntry(FsEntryMetadata::new(Utc::now())),
        //     ),
        // ]
        // .to_vec()
        // .into_iter()
        // .map(|(name, entry)| {
        //     let obj = Object::new();
        //     Reflect::set(&obj, &"name".into(), &name.into()).unwrap();
        //     Reflect::set(
        //         &obj,
        //         &"metadata".into(),
        //         &JsValue::try_from(entry.clone()).unwrap(),
        //     )
        //     .unwrap();
        //     obj
        // })
        // .collect::<Array>();
        // Ok(vec)
    }
}
// TODO:  once we have test metadata, we can test this

    // pub async fn share_with(
    //     &mut self,
    //     _recipient_key: &EcPublicEncryptionKey,
    //     _wrapping_key: &EcEncryptionKey,
    // ) -> Result<(), TombWasmError> {
    //     panic!("not implemented")
    // }

    // pub async fn snapshot(&mut self) -> Result<Snapshot, TombWasmError> {
    //     panic!("not implemented")
    // }

    // // Getters
    


    // /// List the snapshots for the current account
    // /// # Returns
    // /// An array of snapshots of the form:
    // /// ```json
    // ///  [
    // ///     {
    // ///         "id": "uuid",
    // ///        "bucket_id": "uuid",
    // ///      "snapshot_type": "string",
    // ///   "version": "string"
    // ///     }
    // /// ]
    // #[wasm_bindgen(js_name = getSnapshots)]
    // pub async fn get_snapshots(&self) -> JsResult<Array> {
    //     log!("tomb-wasm: get_snapshots()");
    //     let client = &self.banyan_client;
    //     let snapshots = client.get_snapshots().await?;
    //     let snapshots = snapshots
    //         .iter()
    //         .map(|snapshot| {
    //             let value = JsValue::try_from(snapshot.clone()).unwrap();
    //             value
    //         })
    //         .collect::<Array>();
    //     // Ok
    //     Ok(snapshots)
    // }



    // /// List bucket keys for a bucket
    // /// Returns an array of public keys in the form:
    // /// ```json
    // /// [
    // ///   {
    // ///    "id": "uuid",
    // ///    "bucket_id": "uuid",
    // ///    "pem": "string"
    // ///    "approved": "bool"
    // ///  }
    // /// ]
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to list keys for
    // // TODO: Replace with API call
    // #[wasm_bindgen(js_name = getBucketKeys)]
    // pub async fn get_bucket_keys(&self, _bucket_id: &str) -> JsResult<Array> {
    //     log!("tomb-wasm: get_bucket_keys()");
    //     let client = &self.banyan_client;
    //     let keys = client.get_bucket_keys(_bucket_id).await?;
    //     // Convert the keys
    //     let keys = keys
    //         .iter()
    //         .map(|key| {
    //             let value = JsValue::try_from(key.clone()).unwrap();
    //             value
    //         })
    //         .collect::<Array>();
    //     // Ok
    //     Ok(keys)
    // }

    // /// List snapshots for a bucket
    // /// Returns an array of snapshots in the form:
    // /// ```json
    // /// [
    // ///  {
    // ///   "id": "uuid",
    // ///  "bucket_id": "uuid",
    // /// "snapshot_type": "string",
    // /// "version": "string"
    // /// }
    // /// ]
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to list snapshots for
    // #[wasm_bindgen(js_name = getBucketSnapshots)]
    // pub async fn get_bucket_snapshots(&self, _bucket_id: &str) -> JsResult<Array> {
    //     log!("tomb-wasm: get_bucket_snapshots()");
    //     // Call the api
    //     let client = &self.banyan_client;
    //     let snapshots = client.get_bucket_snapshots(_bucket_id).await?;
    //     // Convert the snapshots
    //     let snapshots = snapshots
    //         .iter()
    //         .map(|snapshot| {
    //             let value = JsValue::try_from(snapshot.clone()).unwrap();
    //             value
    //         })
    //         .collect::<Array>();
    //     // Ok
    //     Ok(snapshots)
    // }

    // /// Sync a bucket with the remote
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to sync
    // #[wasm_bindgen(js_name = syncBucket)]
    // pub async fn sync_bucket(&mut self, _bucket_id: &str) -> JsResult<()> {
    //     log!("tomb-wasm: sync_bucket({})", _bucket_id);
    //     // Get the bucket
    //     let bucket = match self.buckets.get_mut(_bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded");
    //         }
    //     };
    //     // Call the api
    //     let client = &self.banyan_client;
    //     client.sync_bucket(&bucket).await?;
    //     Ok(())
    // }

   

    // /// Request access to a bucket
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to request access to
    // /// * `public_key` - The public key to approve access for
    // // TODO: what is the return type?
    // #[wasm_bindgen(js_name = requestBucketAccess)]
    // pub async fn request_bucket_access(
    //     &self,
    //     _bucket_id: String,
    //     _public_key: CryptoKey,
    // ) -> JsResult<()> {
    //     log!("tomb-wasm: request_bucket_access({})", _bucket_id);
    //     // NOTE: leave commented out until we have a way to convert the public key
    //     // Convert the public key
    //     // let public_key = EcPublicEncryptionKey::from(_public_key);
    //     // Call the api
    //     // self.banyan_client
    //     //     .request_bucket_access(&_bucket_id, &public_key)
    //     //     .await?;
    //     // Ok
    //     Ok(())
    // }

    // /// Approve bucket Access (take a UUID of a specific key request)
    // /// Internally this is going to request the public key request, and encrypt the WNFS key with the associated public key, update the metadata and perform a sync
    // /// # Arguments
    // /// * `bucket_key_id` - The id of the key request to approve
    // #[wasm_bindgen(js_name = approveBucketAccess)]
    // pub async fn approve_bucket_access(&self, _bucket_key_id: String) -> JsResult<()> {
    //     log!("tomb-wasm: approve_bucket_access({})", _bucket_key_id);
    //     let bucket_id = "1".to_string();
    //     let bucket = match self.buckets.get(&bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded");
    //         }
    //     };
    //     if bucket.is_locked() {
    //         panic!("Bucket is locked");
    //     };
    //     Ok(())
    // }

    // /* Bucket Interface -- once a bucket is loaded, we can interact with it by its id */
    // /// Unlock a bucket
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to unlock
    // /// * `wrapping_key` - The wrapping key to unlock the bucket with
    // #[wasm_bindgen(js_name = unlock)]
    // pub async fn unlock(&mut self, bucket_id: &str, _wrapping_key: CryptoKey) -> JsResult<()> {
    //     log!("tomb-wasm: unlock({})", bucket_id);
    //     let bucket = match self.buckets.get_mut(bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded");
    //         }
    //     };
    //     if !bucket.is_locked() {
    //         panic!("Bucket is already unlocked");
    //     };
    //     // TODO: Implement with wrapping key
    //     bucket.unlock().await?;
    //     Ok(())
    // }

    // /// List the contents of a bucket at a path
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to list contents for
    // /// * `path` - The path to list contents for
    // /// * `version` - The version to list contents for (optional)
    // /// # Returns
    // /// An array of entries TODO: What form is this?
    // #[wasm_bindgen(js_name = ls)]
    // pub async fn ls(
    //     &self,
    //     bucket_id: &str,
    //     path: &str,
    //     version: Option<String>,
    // ) -> JsResult<Array> {
    //     log!("tomb-wasm: ls({}/{})", bucket_id, path);
    //     let bucket = match self.buckets.get(bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded")
    //         }
    //     };
    //     if bucket.is_locked() {
    //         panic!("Bucket is locked");
    //     };
    //     let path_segments = path.split('/').collect::<Vec<&str>>();
    //     let entries = bucket.ls(path_segments).await?;
    //     let entries = entries
    //         .iter()
    //         .map(|(name, entry)| {
    //             let obj = Object::new();
    //             Reflect::set(&obj, &"name".into(), &name.into()).unwrap();
    //             Reflect::set(
    //                 &obj,
    //                 &"metadata".into(),
    //                 &JsValue::try_from(entry.clone()).unwrap(),
    //             )
    //             .unwrap();
    //             obj
    //         })
    //         .collect::<Array>();
    //     Ok(entries)
    // }

    // /// Snapshot a bucket
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to snapshot
    // /// # Returns
    // /// TODO: What form is this?
    // #[wasm_bindgen(js_name = snapshot)]
    // pub async fn snapshot(&mut self, bucket_id: &str) -> JsResult<()> {
    //     log!("tomb-wasm: snapshot({})", bucket_id);
    //     // Get the bucket
    //     let bucket = match self.buckets.get_mut(bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded");
    //         }
    //     };
    //     // Check if the bucket is unlocked
    //     if bucket.is_locked() {
    //         panic!("Bucket is locked");
    //     };
    //     // Call the bucket
    //     bucket.snapshot().await?;
    //     // Ok
    //     Ok(())
    // }

    // /// Read a file from a bucket
    // ///     Read / Download a File (takes a path to a file inside the bucket, not available for cold only buckets)
    // ///     Allows reading at a version
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to read from
    // /// * `path` - The path to read from
    // /// * `version` - The version to read from (optional)
    // /// # Returns
    // /// TODO: What form is this?
    // /// TODO: Acutal implementation
    // #[wasm_bindgen(js_name = read)]
    // pub async fn read(
    //     &self,
    //     bucket_id: &str,
    //     _path: &str,
    //     _version: Option<String>,
    // ) -> JsResult<()> {
    //     log!("tomb-wasm: read({}/{})", bucket_id, _path);
    //     // Get the bucket
    //     let bucket = match self.buckets.get(bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded");
    //         }
    //     };
    //     // Check if the bucket is unlocked
    //     if bucket.is_locked() {
    //         panic!("Bucket is locked");
    //     };
    //     // Ok
    //     Ok(())
    // }

    // /// Delete a file from a bucket
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to delete from
    // /// * `path` - The path to delete from
    // /// # Returns
    // /// TODO: What form is this?
    // #[wasm_bindgen(js_name = delete)]
    // pub async fn delete(&self, bucket_id: &str, _path: &str) -> JsResult<()> {
    //     log!("tomb-wasm: delete({}/{})", bucket_id, _path);
    //     // Get the bucket
    //     let bucket = match self.buckets.get(bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded");
    //         }
    //     };
    //     // Check if the bucket is unlocked
    //     if bucket.is_locked() {
    //         panic!("Bucket is locked");
    //     };
    //     // Ok
    //     Ok(())
    // }

    // /// Get a file's / folder's metadata from a bucket
    // ///     Get file / folder versions (takes a path to a file or directory inside the bucket)
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to download from
    // /// * `path` - The path to download from
    // /// # Returns
    // /// TODO: What form is this?
    // /// For now we'll just return some sample data
    // /// ```json
    // /// {
    // ///     "id": "uuid",
    // ///     "bucket_id": "uuid",
    // ///     "path": "string",
    // ///     "type": "string",
    // ///     "cid": "string",
    // ///     "size": "u64",
    // ///     "versions": "array",
    // ///     "created_at": "string",
    // ///     "updated_at": "string",
    // /// }
    // #[wasm_bindgen(js_name = getMetadata)]
    // pub async fn get_metadata(&self, bucket_id: &str, path: &str) -> JsResult<JsValue> {
    //     log!("tomb-wasm: get_metadata({}/{})", bucket_id, path);
    //     // Get the bucket
    //     let bucket = match self.buckets.get(bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded");
    //         }
    //     };
    //     // Check if the bucket is unlocked
    //     if bucket.is_locked() {
    //         panic!("Bucket is locked");
    //     };
    //     // Return some sample data
    //     let res = JsValue::from_serde(&serde_json::json!({
    //         "id": "uuid",
    //         "bucket_id": bucket_id,
    //         "path": path,
    //         "type": "file",
    //         "cid": "Qmabc",
    //         "size": 1024,
    //         "versions": [
    //             "1",
    //             "2",
    //             "3"
    //         ],
    //         "created_at": "today",
    //         "updated_at": "today",
    //     }))
    //     .unwrap();
    //     Ok(res)
    // }

    // /// Create a directory in a bucket
    // /// Create directory (takes a path to a non-existent directory)
    // /// Will create parent directories as need to create the file directory
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to create a directory in
    // /// * `path` - The path to create a directory in
    // /// # Returns
    // /// TODO: What form is this?
    // #[wasm_bindgen(js_name = createDirectory)]
    // pub async fn create_directory(&self, bucket_id: &str, _path: &str) -> JsResult<()> {
    //     log!("tomb-wasm: create_directory({}/{})", bucket_id, _path);
    //     // Get the bucket
    //     let bucket = match self.buckets.get(bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded");
    //         }
    //     };
    //     // Check if the bucket is unlocked
    //     if bucket.is_locked() {
    //         panic!("Bucket is locked");
    //     };
    //     // Ok
    //     Ok(())
    // }

    // /// Rename a file or directory in a bucket
    // ///     Rename (tasks a source and destination path, destination must not exist)
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to rename in
    // /// * `source` - The source path to rename
    // /// * `destination` - The destination path to rename to
    // /// # Returns
    // /// TODO: What form is this?
    // #[wasm_bindgen(js_name = rename)]
    // pub async fn rename(&self, bucket_id: &str, _source: &str, _destination: &str) -> JsResult<()> {
    //     log!(
    //         "tomb-wasm: rename({}/{}/{})",
    //         bucket_id,
    //         _source,
    //         _destination
    //     );
    //     // Get the bucket
    //     let bucket = match self.buckets.get(bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded");
    //         }
    //     };
    //     // Check if the bucket is unlocked
    //     if bucket.is_locked() {
    //         panic!("Bucket is locked");
    //     };
    //     // Ok
    //     Ok(())
    // }

    // /// Migrate a file or directory in a bucket to another bucket
    // ///     Migrate (takes a list of files and directories in the current bucket, another OpenedBucket, and a destination in the OpenedBucket to move the data into)
    // /// # Arguments
    // /// * `source_bucket_id` - The id of the bucket to migrate from
    // /// * `destination_bucket_id` - The id of the bucket to migrate to
    // /// * `sources` - The source path to migrate
    // /// * `destinations` - The destination path to migrate to
    // /// # Returns
    // /// TODO: What form is this?
    // #[wasm_bindgen(js_name = migrate)]
    // pub async fn migrate(
    //     &self,
    //     _source_bucket_id: &str,
    //     _destination_bucket_id: &str,
    //     _sources: Array,
    //     _destinations: Array,
    // ) -> JsResult<()> {
    //     log!(
    //         "tomb-wasm: migrate({}/{})",
    //         _source_bucket_id,
    //         _destination_bucket_id
    //     );
    //     // Get the bucket
    //     let source = match self.buckets.get(_source_bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded");
    //         }
    //     };
    //     // Check if the bucket is unlocked
    //     if source.is_locked() {
    //         panic!("Bucket is locked");
    //     };

    //     // Get the bucket
    //     let destination = match self.buckets.get(_destination_bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded");
    //         }
    //     };

    //     // Check if the bucket is unlocked
    //     if destination.is_locked() {
    //         panic!("Bucket is locked");
    //     };

    //     // Ok
    //     Ok(())
    // }

    // /// Upload a file to a bucket
    // /// # Arguments
    // /// * `bucket_id` - The id of the bucket to upload to
    // /// * `path` - The path to upload to
    // /// * `file` - The file to upload
    // /// # Returns
    // /// TODO: What form is this?
    // ///  Upload file (takes a path to a non-existent file, and a ReadableStream)
    // ///    Should produce a promise for a completed upload and a way to track its progress
    // ///    I suspect this is going to be the hardest to implement, I'd save it for last
    // #[wasm_bindgen(js_name = upload)]
    // pub async fn upload(&self, bucket_id: &str, _path: &str, _file: JsValue) -> JsResult<()> {
    //     log!("tomb-wasm: upload({}/{})", bucket_id, _path);
    //     // Get the bucket
    //     let bucket = match self.buckets.get(bucket_id) {
    //         Some(bucket) => bucket,
    //         None => {
    //             panic!("Bucket not loaded");
    //         }
    //     };
    //     // Check if the bucket is unlocked
    //     if bucket.is_locked() {
    //         panic!("Bucket is locked");
    //     };
    //     // Ok
    //     Ok(())
    // }

    // // Snapshot Management

    // /// Purge a snapshot
    // /// # Arguments
    // /// * `snapshot_id` - The id of the snapshot to purge
    // /// # Returns
    // /// TODO: What form is this?
    // #[wasm_bindgen(js_name = purgeSnapshot)]
    // pub async fn purge_snapshot(&self, _snapshot_id: &str) -> JsResult<()> {
    //     // Call the api
    //     let client = &self.banyan_client;
    //     client.purge_snapshot(_snapshot_id).await?;
    //     // Ok
    //     Ok(())
    // }