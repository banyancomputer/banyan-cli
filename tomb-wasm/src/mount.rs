use futures_util::StreamExt;
use gloo::console::log;
use js_sys::{Array, ArrayBuffer, Uint8Array};
use std::convert::TryFrom;
use std::io::Cursor;
use tomb_common::banyan_api::blockstore::BanyanApiBlockStore;
use tomb_common::banyan_api::client::Client;
use tomb_common::banyan_api::models::{bucket::Bucket, metadata::Metadata};
use tomb_common::blockstore::carv2_memory::CarV2MemoryBlockStore as BlockStore;
use tomb_common::metadata::FsMetadata;
use tomb_crypt::prelude::*;
use wasm_bindgen::prelude::*;

// TODO: This should be a config
const BLOCKSTORE_API_HOST: &str = "http://localhost:3002";

use crate::error::TombWasmError;
use crate::types::{WasmBucket, WasmBucketKey, WasmFsMetadataEntry};
use crate::utils::JsResult;

/// Mount point for a Bucket in WASM
/// Enables to call Fs methods on a Bucket, pulling metadata from a remote
#[wasm_bindgen]
pub struct WasmMount {
    /* Remote client */
    /// Client to use for remote calls
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
    /// Initialize a new Wasm callable mount with metadata for a bucket and a client
    pub async fn new(
        wasm_bucket: WasmBucket,
        key: &EcEncryptionKey,
        client: &mut Client,
    ) -> Result<Self, TombWasmError> {
        log!("tomb-wasm: mount/new()/{}", wasm_bucket.id());
        let bucket = Bucket::from(wasm_bucket.clone());
        log!(
            "tomb-wasm: mount/new()/{} - creating blockstores",
            wasm_bucket.id()
        );
        let metadata_blockstore = BlockStore::new().expect("could not create blockstore");
        let content_blockstore = BlockStore::new().expect("could not create blockstore");
        log!(
            "tomb-wasm: mount/new()/{} - creating fs metadata",
            wasm_bucket.id()
        );
        let fs_metadata = FsMetadata::init(key)
            .await
            .expect("could not init fs metadata");
        log!(
            "tomb-wasm: mount/new()/{} - saving fs metadata",
            wasm_bucket.id()
        );
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
        log!(
            "tomb-wasm: mount/pull()/{} - pulling metadata at version {}",
            wasm_bucket.id(),
            metadata_cid
        );
        // Pull the Fs metadata on the matching entry
        let mut stream = metadata
            .pull(client)
            .await
            .expect("could not pull metedata");
        log!(
            "tomb-wasm: mount/pull()/{} - reading metadata stream",
            wasm_bucket.id()
        );
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }
        log!(
            "tomb-wasm: mount/pull()/{} - creating metadata blockstore",
            wasm_bucket.id()
        );
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
            log!(
                "tomb-wasm: mount/sync()/{} - bucket is locked",
                self.bucket.id.to_string()
            );
            panic!("Bucket is locked");
        };
        log!(
            "tomb-wasm: mount/sync()/{} - saving changes",
            self.bucket.id.to_string()
        );

        if self.dirty() {
            log!(
                "tomb-wasm: mount/sync()/{} - saving changes to fs",
                self.bucket.id.to_string()
            );
            let _ = self
                .fs_metadata
                .as_mut()
                .unwrap()
                .save(&self.metadata_blockstore, &self.content_blockstore)
                .await;
        } else {
            log!(
                "tomb-wasm: mount/sync()/{} - no changes to fs",
                self.bucket.id.to_string()
            );
        }

        log!(
            "tomb-wasm: mount/sync()/{} - pushing changes",
            self.bucket.id.to_string()
        );

        let root_cid = self
            .fs_metadata
            .as_ref()
            .unwrap()
            .root_cid(&self.metadata_blockstore)
            .await
            .expect("could not get root cid");
        let metadata_cid = self
            .fs_metadata
            .as_ref()
            .unwrap()
            .metadata_cid(&self.metadata_blockstore)
            .await
            .expect("could not get metadata cid");

        log!(
            "tomb-wasm: mount/sync()/{} - pushing metadata at version {}",
            self.bucket.id.to_string(),
            metadata_cid.to_string()
        );
        log!(
            "tomb-wasm: mount/sync()/{} - pushing root at version {}",
            self.bucket.id.to_string(),
            root_cid.to_string()
        );

        let (metadata, storage_ticket) = Metadata::push(
            self.bucket.id,
            root_cid.to_string(),
            metadata_cid.to_string(),
            0,
            // This may lint as an error but it is not
            Cursor::new(self.metadata_blockstore.get_data()),
            &mut self.client,
        )
        .await
        .expect("could not push metadata");

        assert_eq!(metadata.metadata_cid, metadata_cid.to_string());
        assert_eq!(metadata.root_cid, root_cid.to_string());
        let metadata_id = metadata.id;
        self.metadata = Some(metadata);

        match storage_ticket {
            Some(storage_ticket) => {
                log!(
                    "tomb-wasm: mount/sync()/{} - storage ticket returned",
                    self.bucket.id.to_string()
                );
                storage_ticket
                    .clone()
                    .create_grant(&mut self.client)
                    .await
                    .expect("could not create grant");
                let content = Cursor::new(self.metadata_blockstore.get_data());
                storage_ticket
                    .clone()
                    .upload_content(metadata_id, content, &mut self.client)
                    .await
                    .expect("could not upload content");
            }
            None => {
                log!(
                    "tomb-wasm: mount/sync()/{} - no storage ticket returned no content to upload",
                    self.bucket.id.to_string()
                );
            }
        }

        self.dirty = false;
        log!(
            "tomb-wasm: mount/sync()/{} - synced",
            self.bucket.id.to_string()
        );
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
        log!(
            "tomb-wasm: mount/unlock()/{} - unlocking",
            self.bucket.id.to_string()
        );
        // Get the metadata
        let fs_metadata = FsMetadata::unlock(key, &self.metadata_blockstore)
            .await
            .expect("could not unlock fs metadata");

        log!(
            "tomb-wasm: mount/unlock()/{} - checking versioning",
            self.bucket.id.to_string()
        );

        let metadata_cid = fs_metadata
            .metadata_cid(&self.metadata_blockstore)
            .await
            .expect("could not get metadata cid");
        let root_cid = fs_metadata
            .root_cid(&self.metadata_blockstore)
            .await
            .expect("could not get root cid");
        let metadata = self.metadata.as_ref().unwrap();

        assert_eq!(metadata_cid.to_string(), metadata.metadata_cid);
        assert_eq!(root_cid.to_string(), metadata.root_cid);

        log!(
            "tomb-wasm: mount/unlock()/{} - unlocked",
            self.bucket.id.to_string()
        );
        // Ok
        self.locked = false;
        self.fs_metadata = Some(fs_metadata);
        Ok(self)
    }
}

#[wasm_bindgen]
impl WasmMount {
    /// Returns whether or not the bucket is locked
    pub fn locked(&self) -> bool {
        self.locked
    }

    /// Returns whether or not the bucket is dirty
    /// - when a file or dir is changed
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Ls the bucket at a path
    /// # Arguments
    /// * `path_segments` - The path to ls (as an Array)
    /// # Returns
    /// The an Array of objects in the form of:
    /// ```json
    /// {
    ///    "name": "string",
    ///   "entry_type": "string", (file | dir)
    ///  "metadata": {
    ///    "created": 0,
    ///   "modified": 0,
    ///  "size": 0,
    /// "cid": "string"
    /// }
    /// # Errors
    /// * `Bucket is locked` - If the bucket is locked
    pub async fn ls(&mut self, path_segments: Array) -> JsResult<Array> {
        // Read the array as a Vec<String>
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!(
            "tomb-wasm: mount/ls/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        log!(
            "tomb-wasm: mount/ls/{}/{} - getting entries",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );
        // Get the entries
        let fs_metadata_entries = self
            .fs_metadata
            .as_ref()
            .unwrap()
            .ls(path_segments, &self.metadata_blockstore)
            .await
            .expect("could not ls");

        log!(
            "tomb-wasm: mount/ls/{} - mapping entries",
            self.bucket.id.to_string()
        );
        // Map the entries back to JsValues
        let entries = fs_metadata_entries
            .iter()
            .map(|entry| {
                JsValue::try_from(WasmFsMetadataEntry(entry.clone())).unwrap()
            })
            .collect::<Array>();

        // Ok
        Ok(entries)
    }

    /// Mkdir
    /// # Arguments
    /// * `path_segments` - The path to mkdir (as an Array)
    /// # Returns
    /// Promise<void> in js speak
    /// # Errors
    /// * `Bucket is locked` - If the bucket is locked
    /// * `Could not mkdir` - If the mkdir fails
    /// * `Could not sync` - If the sync fails
    pub async fn mkdir(&mut self, path_segments: Array) -> JsResult<()> {
        // Read the array as a Vec<String>
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!(
            "tomb-wasm: mount/mkdir/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        log!(
            "tomb-wasm: mount/mkdir/{}/{} - mkdir",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );
        self.fs_metadata
            .as_mut()
            .unwrap()
            .mkdir(path_segments, &self.metadata_blockstore)
            .await
            .expect("could not mkdir");

        log!(
            "tomb-wasm: mount/mkdir/{}/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );
        self.dirty = true;
        self.sync().await.expect("could not sync");

        // Ok
        Ok(())
    }

    /// Add a file
    /// # Arguments
    /// * `path_segments` - The path to add to (as an Array)
    /// * `content_buffer` - The content to add (as an ArrayBuffer)
    /// # Returns
    /// Promise<void> in js speak
    /// # Errors
    /// * `Bucket is locked` - If the bucket is locked
    /// * `Could not add` - If the add fails
    /// * `Could not sync` - If the sync fails
    pub async fn add(&mut self, path_segments: Array, content_buffer: ArrayBuffer) -> JsResult<()> {
        // Read the array as a Vec<String>
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!(
            "tomb-wasm: mount/add/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        let content = Uint8Array::new(&content_buffer).to_vec();

        self.fs_metadata
            .as_mut()
            .unwrap()
            .add(
                path_segments,
                content,
                &self.metadata_blockstore,
                &self.content_blockstore,
            )
            .await
            .expect("could not add");

        log!(
            "tomb-wasm: mount/add/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );
        self.dirty = true;
        self.sync().await.expect("could not sync");

        // Ok
        Ok(())
    }

    // TODO: Attaching approved keys to the metadata push
    /// Share with
    /// # Arguments
    /// * `recipient_key` - This is a Bucket key, a json value in form
    /// ```json
    /// {
    ///     "id": "uuid",
    ///     "bucket_id": "uuid",
    ///     "pem": "string",
    ///     "approved": "bool"
    /// }
    /// ```
    /// # Returns
    /// Promise<void> in js speak
    #[wasm_bindgen(js_name = shareWith)]
    pub async fn share_with(&mut self, recipient_key: JsValue) -> JsResult<()> {
        log!("tomb-wasm: mount/share_with/{}", self.bucket.id.to_string());
        let recipient_key =
            WasmBucketKey::try_from(recipient_key).expect("could not parse bucket key");

        let recipient_key = &recipient_key.0.pem;
        log!(
            "tomb-wasm: mount/share_with/{} - importing key",
            recipient_key.clone()
        );
        let recipient_key = &EcPublicEncryptionKey::import(recipient_key.as_bytes())
            .await
            .expect("could not import key");

        if self.locked() {
            panic!("Bucket is locked");
        };

        self.fs_metadata
            .as_mut()
            .unwrap()
            .share_with(recipient_key, &self.metadata_blockstore)
            .await
            .expect("could not share with");

        log!(
            "tomb-wasm: mount/share_with/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );

        self.sync().await.expect("could not sync");

        // Ok
        Ok(())
    }

    /// Snapshot a mounted bucket
    /// # Returns
    /// A Promise<void> in js speak
    /// # Errors
    /// * "missing metadata" - If the metadata is missing
    /// * "could not snapshot" - If the snapshot fails
    #[wasm_bindgen(js_name = snapshot)]
    pub async fn snapshot(&mut self) -> JsResult<()> {
        log!("tomb-wasm: mount/snapshot/{}", self.bucket.id.to_string());
        // Get the bucket
        let metadata = self.metadata.as_ref();
        metadata
            .expect("missing metadata")
            .snapshot(&mut self.client)
            .await
            .expect("could not snapshot");
        // Ok
        Ok(())
    }

    /// Read a file from a mounted bucket
    ///     Read / Download a File (takes a path to a file inside the bucket, not available for cold only buckets)
    ///     Allows reading at a version
    /// # Arguments
    /// * `path_segments` - The path to read from (as an Array)
    /// * `version` - The version to read from (optional)
    /// # Returns
    /// A Promise<ArrayBuffer> in js speak
    #[wasm_bindgen(js_name = readBytes)]
    pub async fn read_bytes(
        &mut self,
        path_segments: Array,
        _version: Option<String>,
    ) -> JsResult<ArrayBuffer> {
        // Read the array as a Vec<String>
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!(
            "tomb-wasm: mount/read_bytes/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        let mut banyan_api_blockstore_client = self.client.clone();
        banyan_api_blockstore_client
            .with_remote(BLOCKSTORE_API_HOST)
            .expect("could not create blockstore client");
        let banyan_api_blockstore = BanyanApiBlockStore::from(banyan_api_blockstore_client);

        let vec = self
            .fs_metadata
            .as_mut()
            .unwrap()
            .read(
                path_segments,
                &self.metadata_blockstore,
                &banyan_api_blockstore,
            )
            .await
            .expect("could not read bytes");

        let bytes = vec.into_boxed_slice();
        let array = Uint8Array::from(&bytes[..]);
        Ok(array.buffer())
    }
}

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
