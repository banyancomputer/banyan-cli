use futures_util::StreamExt;
use gloo::console::log;
use js_sys::Array;
use std::io::Cursor;
use std::convert::TryFrom;
use tomb_common::banyan_api::client::Client;
use tomb_common::banyan_api::models::{bucket::Bucket, metadata::Metadata};
use tomb_common::blockstore::carv2_memory::CarV2MemoryBlockStore as BlockStore;
use tomb_common::metadata::FsMetadata;
use tomb_crypt::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::FileReader;

use crate::error::TombWasmError;
use crate::types::{WasmBucket, WasmFsMetadataEntry};
use crate::utils::JsResult;

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
    metadata: Option<Metadata>,
    /// Whether or not the bucket is locked
    locked: bool,
    /// Whether or not a change requires a call to save
    dirty: bool,

    /* Fs Exposure  */
    /// Encrypted metadata within a local memory blockstore
    metadata_blockstore: BlockStore,
    content_blockstore: BlockStore,

    /// Fs Metadata
    fs_metadata: Option<FsMetadata>,
}

impl WasmMount {
    pub async fn new(
        wasm_bucket: WasmBucket,
        key: &EcEncryptionKey,
        client: &mut Client,
    ) -> Result<Self, TombWasmError> {
        log!("tomb-wasm: mount/new()/{}", wasm_bucket.id());
        let bucket = Bucket::from(wasm_bucket.clone());
        log!("tomb-wasm: mount/new()/{} - creating blockstores", wasm_bucket.id());
        let metadata_blockstore = BlockStore::new().expect("could not create blockstore");
        let content_blockstore = BlockStore::new().expect("could not create blockstore");
        log!("tomb-wasm: mount/new()/{} - creating fs metadata", wasm_bucket.id());
        let fs_metadata = FsMetadata::init(key)
            .await
            .expect("could not init fs metadata");
        log!("tomb-wasm: mount/new()/{} - saving fs metadata", wasm_bucket.id());
        let mut mount = Self {
            client: client.to_owned(),
            bucket,
            metadata: None,
            locked: false,
            dirty: true,
            metadata_blockstore,
            content_blockstore,
            fs_metadata: Some(fs_metadata),
        };

        log!("tomb-wasm: mount/new()/{} - syncing", wasm_bucket.id());
        mount.sync().await.expect("could not sync");
        // Ok
        Ok(mount)
    }
    /// Initialize a new Wasm callable mount with metadata for a bucket and a client
    pub async fn pull(wasm_bucket: WasmBucket, client: &mut Client) -> Result<Self, TombWasmError> {
        log!("tomb-wasm: mount/pull()/{}", wasm_bucket.id());
        // Get the underlying bucket
        let bucket = Bucket::from(wasm_bucket.clone());
        // Get the metadata associated with the bucket
        let metadata = Metadata::read_current(bucket.id, client)
            .await
            .map_err(|_| TombWasmError::unknown_error())?;
        let metadata_cid = metadata.metadata_cid.clone();
        log!("tomb-wasm: mount/pull()/{} - pulling metadata at version {}", wasm_bucket.id(), metadata_cid);
        // Pull the Fs metadata on the matching entry
        let mut stream = metadata
            .pull(client)
            .await
            .expect("could not pull metedata");
        log!("tomb-wasm: mount/pull()/{} - reading metadata stream", wasm_bucket.id());
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }
        log!("tomb-wasm: mount/pull()/{} - creating metadata blockstore", wasm_bucket.id());
        let metadata_blockstore =
            BlockStore::try_from(data).expect("could not create metadata as blockstore");
        let content_blockstore = BlockStore::new().expect("could not create blockstore");

        log!("tomb-wasm: mount/pull()/{} - pulled", wasm_bucket.id());

        // Ok
        Ok(Self {
            client: client.to_owned(),
            bucket,
            metadata: Some(metadata.to_owned()),
            locked: true,
            dirty: false,
            metadata_blockstore,
            content_blockstore,
            fs_metadata: None,
        })
    }

    /// Sync the current fs_metadata with the remote
    pub async fn sync(&mut self) -> Result<(), TombWasmError> {
        log!("tomb-wasm: mount/sync()/{}", self.bucket.id.to_string());
        // Check if the bucket is locked
        if self.locked() {
            log!("tomb-wasm: mount/sync()/{} - bucket is locked", self.bucket.id.to_string());
            panic!("Bucket is locked");
        };
        log!("tomb-wasm: mount/sync()/{} - saving changes", self.bucket.id.to_string());
        if !self.dirty() {
            log!("tomb-wasm: mount/sync()/{} - no changes to save", self.bucket.id.to_string());
            panic!("Bucket is clean");
        }
        let _ = self.fs_metadata
            .as_mut()
            .unwrap()
            .save(&self.metadata_blockstore, &self.content_blockstore)
            .await;
        
        log!("tomb-wasm: mount/sync()/{} - pushing changes", self.bucket.id.to_string());

        let root_cid = self.fs_metadata.as_ref().unwrap().root_cid(&self.metadata_blockstore).await.expect("could not get root cid");
        let metadata_cid = self.fs_metadata.as_ref().unwrap().metadata_cid(&self.metadata_blockstore).await.expect("could not get metadata cid");
        
        log!("tomb-wasm: mount/sync()/{} - pushing metadata at version {}", self.bucket.id.to_string(), metadata_cid.to_string());
        log!("tomb-wasm: mount/sync()/{} - pushing root at version {}", self.bucket.id.to_string(), root_cid.to_string());

        let (metadata, _) = Metadata::push(
            self.bucket.id,
            root_cid.to_string(),
            metadata_cid.to_string(),
            0,
            // This may lint as an error but it is not
            Cursor::new(self.metadata_blockstore.get_data()), 
            &mut self.client,
        ).await.expect("could not push metadata");

        assert_eq!(metadata.metadata_cid, metadata_cid.to_string());
        assert_eq!(metadata.root_cid, root_cid.to_string());

        self.metadata = Some(metadata);
        self.dirty = false;

        log!("tomb-wasm: mount/sync()/{} - synced", self.bucket.id.to_string());
        // Ok
        Ok(())
    }

    /// Unlock the current fs_metadata
    pub async fn unlock(mut self, key: &EcEncryptionKey) -> Result<Self, TombWasmError> {
        log!("tomb-wasm: mount/unlock()/{}", self.bucket.id.to_string());
        // Check if the bucket is already unlocked
        if !self.locked() {
            panic!("Bucket is already unlocked");
        };
        log!("tomb-wasm: mount/unlock()/{} - unlocking", self.bucket.id.to_string());
        // Get the metadata
        let fs_metadata = FsMetadata::unlock(key, &self.metadata_blockstore)
            .await
            .expect("could not unlock fs metadata");

        log!("tomb-wasm: mount/unlock()/{} - checking versioning", self.bucket.id.to_string());

        let metadata_cid = fs_metadata.metadata_cid(&self.metadata_blockstore).await.expect("could not get metadata cid");
        let root_cid = fs_metadata.root_cid(&self.metadata_blockstore).await.expect("could not get root cid");
        let metadata = self.metadata.as_ref().unwrap();

        assert_eq!(metadata_cid.to_string(), metadata.metadata_cid);
        assert_eq!(root_cid.to_string(), metadata.root_cid);

        log!("tomb-wasm: mount/unlock()/{} - unlocked", self.bucket.id.to_string());
        // Ok
        self.locked = false;
        self.fs_metadata = Some(fs_metadata);
        Ok(self)
    }
}

#[wasm_bindgen]
impl WasmMount {
    pub fn locked(&self) -> bool {
        self.locked == true
    }

    pub fn dirty(&self) -> bool {
        self.dirty == true
    }

    pub async fn ls(&mut self, path_segments: Array) -> JsResult<Array> {
        // Read the array as a Vec<String>
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!("tomb-wasm: mount/ls/{}/{}", self.bucket.id.to_string(), &path_segments.join("/"));
        
        if self.locked() {
            panic!("Bucket is locked");
        };

        log!("tomb-wasm: mount/ls/{}/{} - getting entries", self.bucket.id.to_string(), &path_segments.join("/"));
        // Get the entries
        let fs_metadata_entries = self
            .fs_metadata
            .as_ref()
            .unwrap()
            .ls(path_segments, &self.metadata_blockstore)
            .await
            .expect("could not ls");

        log!("tomb-wasm: mount/ls/{} - mapping entries", self.bucket.id.to_string());
        // Map the entries back to JsValues
        let entries = fs_metadata_entries
            .iter()
            .map(|entry| {
                let value = JsValue::try_from(WasmFsMetadataEntry(entry.clone())).unwrap();
                value
            })
            .collect::<Array>();

        // Ok
        Ok(entries)
    }

    /// Mkdir
    pub async fn mkdir(&mut self, path_segments: Array) -> JsResult<()> {
        // Read the array as a Vec<String>
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!("tomb-wasm: mount/mkdir/{}/{}", self.bucket.id.to_string(), &path_segments.join("/"));

        if self.locked() {
            panic!("Bucket is locked");
        };
        
        log!("tomb-wasm: mount/mkdir/{}/{} - mkdir", self.bucket.id.to_string(), &path_segments.join("/"));
        self.fs_metadata
            .as_mut()
            .unwrap()
            .mkdir(path_segments, &self.metadata_blockstore)
            .await
            .expect("could not mkdir");

        log!("tomb-wasm: mount/mkdir/{}/{} - dirty, syncing changes", self.bucket.id.to_string());
        self.dirty = true;
        self.sync().await.expect("could not sync");
        
        // Ok
        Ok(())
    }

    // /// Add a file
    // pub async fn add(
    //     &mut self,
    //     path_segments: Array,
    //     reader: FileReader,
    // ) -> JsResult<()> {
    //     // Read the array as a Vec<String>
    //     let path_segments = path_segments
    //         .iter()
    //         .map(|s| s.as_string().unwrap())
    //         .collect::<Vec<String>>();

    //     log!("tomb-wasm: mount/add/{}/{}", self.bucket.id.to_string(), &path_segments.join("/"));

    //     if self.locked() {
    //         panic!("Bucket is locked");
    //     };

    //     let result = reader.result().expect("could not get result from reader");
    //     let content = result.as_string().expect("could not get string from result");



    //     log!("tomb-wasm: mount/add/{}/{} - add", self.bucket.id.to_string(), &path_segments.join("/"));
    //     self.fs_metadata
    //         .as_mut()
    //         .unwrap()
    //         .add(path_segments, content, &self.metadata_blockstore, &self.content_blockstore)
    //         .await
    //         .expect("could not add");

    //     log!("tomb-wasm: mount/add/{}/{} - dirty, syncing changes", self.bucket.id.to_string(), &path_segments.join("/"));
    //     self.dirty = true;
    //     self.sync().await.expect("could not sync");
        
    //     // Ok
    //     Ok(())
    // }
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
