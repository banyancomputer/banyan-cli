use crate::wasm::WasmBucketMetadata;
use futures_util::StreamExt;
use gloo::console::log;
use js_sys::{Array, ArrayBuffer, Uint8Array};
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::io::Cursor;
use tomb_crypt::prelude::*;
use wasm_bindgen::prelude::*;
use wnfs::private::PrivateNode;

use crate::{
    api::{
        client::Client,
        models::{
            bucket::Bucket, bucket_key::BucketKey, metadata::Metadata, snapshot::Snapshot,
            storage_ticket::StorageTicket,
        },
        requests::staging::upload::content::UploadContent,
    },
    blockstore::{BanyanApiBlockStore, CarV2MemoryBlockStore as BlockStore, RootedBlockStore},
    filesystem::metadata::FsMetadata,
    wasm::{TombResult, TombWasmError, WasmBucket, WasmFsMetadataEntry, WasmSnapshot},
};

/// Mount point for a Bucket in WASM
///
/// Enables to call Fs methods on a Bucket, pulling metadata from a remote
#[derive(Debug)]
#[wasm_bindgen]
pub struct WasmMount {
    client: Client,
    bucket: Bucket,

    metadata: Option<Metadata>,
    fs_metadata: Option<FsMetadata>,

    locked: bool,
    /// Whether or not a change requires a call to save
    dirty: bool,

    /// Whether or not data has been appended to the content blockstore
    append: bool,

    /// Deleted Block CIDs
    deleted_block_cids: BTreeSet<String>,

    metadata_blockstore: BlockStore,
    content_blockstore: BlockStore,
}

impl WasmMount {
    /// Initialize a new Wasm callable mount with metadata for a bucket and a client
    pub async fn new(
        wasm_bucket: WasmBucket,
        key: &EcEncryptionKey,
        client: &Client,
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
            append: false,

            deleted_block_cids: BTreeSet::new(),
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
            .map_err(|err| TombWasmError(format!("unable to read current metadata: {err}")))?;

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
            append: false,
            deleted_block_cids: BTreeSet::new(),

            metadata_blockstore,
            content_blockstore,
            fs_metadata: None,
        })
    }

    /// Refresh the current fs_metadata with the remote
    pub async fn refresh(&mut self, key: &EcEncryptionKey) -> Result<(), TombWasmError> {
        let bucket_id = self.bucket.id;

        // Get the metadata associated with the bucket
        let metadata = Metadata::read_current(bucket_id, &mut self.client)
            .await
            .map_err(|err| TombWasmError(format!("unable to read current metadata: {err}")))?;

        let metadata_cid = metadata.metadata_cid.clone();
        log!(
            "tomb-wasm: mount/pull()/{} - pulling metadata at version {}",
            self.bucket.id.to_string(),
            metadata_cid
        );

        // Pull the Fs metadata on the matching entry
        let mut stream = metadata
            .pull(&mut self.client)
            .await
            .expect("could not pull metedata");

        log!(
            "tomb-wasm: mount/pull()/{} - reading metadata stream",
            self.bucket.id.to_string()
        );

        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }

        log!(
            "tomb-wasm: mount/pull()/{} - creating metadata blockstore",
            self.bucket.id.to_string()
        );

        let metadata_blockstore =
            BlockStore::try_from(data).expect("could not create metadata as blockstore");
        let content_blockstore = BlockStore::new().expect("could not create blockstore");

        self.metadata = Some(metadata.to_owned());
        self.metadata_blockstore = metadata_blockstore;
        self.content_blockstore = content_blockstore;
        self.dirty = false;
        self.append = false;
        self.fs_metadata = None;

        log!(
            "tomb-wasm: mount/pull()/{} - pulled",
            self.bucket.id.to_string()
        );
        self.unlock(key).await.expect("could not unlock");
        // Ok
        Ok(())
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

        log!(format!("tomb-wasm: self.dirty: {}", self.dirty()));
        if self.dirty() {
            log!(
                "tomb-wasm: mount/sync()/{} - saving changes to fs",
                self.bucket.id.to_string()
            );
            let result = self
                .fs_metadata
                .as_mut()
                .unwrap()
                .save(&self.metadata_blockstore, &self.content_blockstore)
                .await;
            log!(format!(
                "tomb-wasm: mount/sync()/{} - save result: {:?}",
                self.bucket.id.to_string(),
                result
            ));
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
            .content_blockstore
            .get_root()
            .expect("could not get root cid");
        let metadata_cid = self
            .metadata_blockstore
            .get_root()
            .expect("could not get metadata cid");
        log!(
            "tomb-wasm: mount/sync()/{} - pushing metadata at version {}",
            self.bucket.id.to_string(),
            metadata_cid.to_string()
        );
        log!(format!(
            "tomb-wasm: mount/sync()/{} - pushing root at version {}",
            self.bucket.id, root_cid,
        ));
        // Assume that the metadata is always at least as big as the content
        let mut data_size = 0;
        if self.append {
            data_size = self.content_blockstore.data_size();
        }
        log!(
            "tomb-wasm: mount/sync()/{} - content size difference {data_size}",
            self.bucket.id.to_string(),
            metadata_cid.to_string(),
            data_size
        );
        let (metadata, host, authorization) = Metadata::push(
            self.bucket.id,
            root_cid.to_string(),
            metadata_cid.to_string(),
            data_size,
            self.fs_metadata
                .as_ref()
                .expect("no fs metadata")
                .share_manager
                .public_fingerprints(),
            // This may lint as an error but it is not
            self.deleted_block_cids.clone(),
            Cursor::new(self.metadata_blockstore.get_data()),
            &mut self.client,
        )
        .await
        .expect("could not push metadata");

        assert_eq!(metadata.root_cid, root_cid.to_string());
        assert_eq!(metadata.metadata_cid, metadata_cid.to_string());
        let metadata_id = metadata.id;
        self.metadata = Some(metadata);

        match (host, authorization) {
            // New storage ticket
            (Some(host), Some(authorization)) => {
                // First create a grant
                StorageTicket {
                    host: host.clone(),
                    authorization,
                }
                .create_grant(&mut self.client)
                .await
                .map_err(|err| TombWasmError(format!("unable to register storage grant: {err}")))?;

                // Then perform upload
                self.content_blockstore
                    .upload(host, metadata_id, &mut self.client)
                    .await
                    .map_err(|err| {
                        TombWasmError(format!("created grant but unable to upload: {err}"))
                    })?;
            }
            // Already granted, still upload
            (Some(host), None) => {
                self.content_blockstore
                    .upload(host, metadata_id, &mut self.client)
                    .await
                    .map_err(|err| {
                        TombWasmError(format!("no grant needed but unable to upload: {err}"))
                    })?;
            }
            // No uploading required
            _ => {
                log!("tomb-wasm: mount/sync()/ - no need to push content");
            }
        }

        self.dirty = false;
        self.append = false;

        log!(format!(
            "tomb-wasm: mount/sync()/{} - synced",
            self.bucket.id.to_string()
        ));

        Ok(())
    }

    /// Unlock the current fs_metadata
    pub async fn unlock(&mut self, key: &EcEncryptionKey) -> Result<(), TombWasmError> {
        log!(format!("tomb-wasm: mount/unlock()/{}", self.bucket.id));

        // Check if the bucket is already unlocked
        if !self.locked() {
            return Ok(());
        }

        log!(format!(
            "tomb-wasm: mount/unlock()/{} - unlocking",
            self.bucket.id,
        ));

        log!(format!(
            "tomb-wasm: mount/unlock()/{} - checking versioning",
            self.bucket.id,
        ));
        let Some(metadata_cid) = self.metadata_blockstore.get_root() else {
            return Err(TombWasmError("unable to retrieve metadata CID".to_string()));
        };
        // Default to the metadata cid if its not present
        // Remember this is the CID of the IPLD, which will be the same in both cases.
        // TODO change this if we stop using the same CIDs in both.
        let root_cid = self.content_blockstore.get_root().unwrap_or(metadata_cid);

        let metadata = self.metadata.as_ref().unwrap();

        assert_eq!(metadata_cid.to_string(), metadata.metadata_cid);
        assert_eq!(root_cid.to_string(), metadata.root_cid);

        // Now try unlocking the metadata
        let fs_metadata = FsMetadata::unlock(key, &self.metadata_blockstore)
            .await
            .map_err(|err| TombWasmError(format!("could not unlock fs metadata: {err}")))?;

        log!(format!(
            "tomb-wasm: mount/unlock()/{} - unlocked",
            self.bucket.id,
        ));

        self.locked = false;
        self.fs_metadata = Some(fs_metadata);

        Ok(())
    }
}

#[wasm_bindgen]
impl WasmMount {
    /// Returns whether or not the bucket is dirty (this will be true when a file or directory has
    /// been changed).
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Returns whether or not the bucket is locked
    pub fn locked(&self) -> bool {
        self.locked
    }

    /// Returns the Metadata for the bucket
    pub fn metadata(&self) -> TombResult<WasmBucketMetadata> {
        let metadata = self.metadata.as_ref().expect("no metadata");
        let wasm_bucket_metadata = WasmBucketMetadata(metadata.clone());
        Ok(wasm_bucket_metadata)
    }

    /// List the contents of the bucket at a provided path
    ///
    /// # Arguments
    ///
    /// * `path_segments` - The path to ls (as an Array)
    ///
    /// # Returns
    ///
    /// The an Array of objects in the form of:
    ///
    /// ```json
    /// [
    ///   {
    ///     "name": "string",
    ///     "entry_type": "(file | dir)"
    ///     "metadata": {
    ///       "created": 0,
    ///       "modified": 0,
    ///       "size": 0,
    ///       "cid": "string"
    ///     }
    ///   }
    /// ]
    /// ```
    ///
    /// # Errors
    ///
    /// * `Bucket is locked` - If the bucket is locked
    pub async fn ls(&mut self, path_segments: Array) -> TombResult<Array> {
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
            return Err(TombWasmError(
                "unable to list directory contents of a locked bucket".to_string(),
            )
            .into());
        };

        log!(format!(
            "tomb-wasm: mount/ls/{}/{} - getting entries",
            self.bucket.id,
            &path_segments.join("/")
        ));

        // Get the entries
        let fs_metadata_entries = self
            .fs_metadata
            .as_ref()
            .unwrap()
            .ls(&path_segments, &self.metadata_blockstore)
            .await
            .map_err(|err| TombWasmError(format!("could not list directory entries: {err}")))?;

        log!(format!(
            "tomb-wasm: mount/ls/{} - mapping entries",
            self.bucket.id,
        ));

        // Map the entries back to JsValues
        fs_metadata_entries
            .iter()
            .map(|entry| {
                let wasm_fs_metadata_entry = WasmFsMetadataEntry::from(entry.clone());
                JsValue::try_from(wasm_fs_metadata_entry).map_err(|err| {
                    TombWasmError(format!(
                        "unable to convert directory entries to JS objects: {err:?}"
                    ))
                    .into()
                })
            })
            .collect()
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
    pub async fn mkdir(&mut self, path_segments: Array) -> TombResult<()> {
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
            .mkdir(&path_segments, &self.metadata_blockstore)
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

    /// Write a file
    /// # Arguments
    /// * `path_segments` - The path to write to (as an Array)
    /// * `content_buffer` - The content to write (as an ArrayBuffer)
    /// # Returns
    /// Promise<void> in js speak
    /// # Errors
    /// * `Bucket is locked` - If the bucket is locked
    /// * `Could not add` - If the add fails
    /// * `Could not sync` - If the sync fails
    pub async fn write(
        &mut self,
        path_segments: Array,
        content_buffer: ArrayBuffer,
    ) -> TombResult<()> {
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
            .write(
                &path_segments,
                &self.metadata_blockstore,
                &self.content_blockstore,
                content,
            )
            .await
            .expect("could not add");
        log!(
            "tomb-wasm: mount/add/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );
        self.dirty = true;
        self.append = true;

        self.sync().await.expect("could not sync");

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
    ) -> TombResult<Uint8Array> {
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

        let mut api_blockstore_client = self.client.clone();
        api_blockstore_client
            .with_remote(self.client.remote_data.as_str())
            .expect("could not create blockstore client");
        let api_blockstore = BanyanApiBlockStore::from(api_blockstore_client);

        let fs = self.fs_metadata.as_mut().unwrap();

        let node = fs
            .get_node(&path_segments, &self.metadata_blockstore)
            .await
            .expect("cant access fs")
            .expect("no node at this path");
        if let PrivateNode::File(file) = node {
            let cids = file
                .get_cids(&fs.forest, &self.metadata_blockstore)
                .await
                .expect("cant get cids");
            api_blockstore.find_cids(cids).await.ok();
        }

        log!(format!(
            "tomb-wasm: running fs_get_node @ {:?}",
            path_segments
        ));

        let vec = fs
            .read(&path_segments, &self.metadata_blockstore, &api_blockstore)
            .await
            .expect("could not read bytes");

        let bytes = vec.into_boxed_slice();
        let array = Uint8Array::from(&bytes[..]);
        Ok(array)
    }

    // TODO: Get metadata on node

    /// Mv a file or directory
    /// # Arguments
    /// * `from_path_segments` - The path to mv from (as an Array)
    /// * `to_path_segments` - The path to mv to (as an Array)
    /// # Returns
    /// Promise<void> in js speak
    /// # Errors
    /// * `Bucket is locked` - If the bucket is locked
    /// * `Could not mv` - If the mv fails, such as if the path does not exist in the bucket
    /// * `Could not sync` - If the sync fails
    pub async fn mv(
        &mut self,
        from_path_segments: Array,
        to_path_segments: Array,
    ) -> TombResult<()> {
        let from_path_segments = from_path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();
        let to_path_segments = to_path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!(
            "tomb-wasm: mount/mv/{}/{} => {}",
            self.bucket.id.to_string(),
            &from_path_segments.join("/"),
            &to_path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        self.fs_metadata
            .as_mut()
            .unwrap()
            .mv(
                &from_path_segments,
                &to_path_segments,
                &self.metadata_blockstore,
                &self.content_blockstore,
            )
            .await
            .expect("could not mv");

        log!(
            "tomb-wasm: mount/mv/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );
        self.dirty = true;
        self.sync().await.expect("could not sync");

        // Ok
        Ok(())
    }

    /// Rm a file or directory
    /// # Arguments
    /// * `path_segments` - The path to rm (as an Array)
    /// # Returns
    /// Promise<void> in js speak
    /// # Errors
    /// * `Bucket is locked` - If the bucket is locked
    /// * `Could not rm` - If the rm fails
    /// * `Could not sync` - If the sync fails
    pub async fn rm(&mut self, path_segments: Array) -> TombResult<()> {
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().unwrap())
            .collect::<Vec<String>>();

        log!(
            "tomb-wasm: mount/rm/{}/{}",
            self.bucket.id.to_string(),
            path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        let fs = self.fs_metadata.as_mut().unwrap();

        let node = fs
            .get_node(&path_segments, &self.metadata_blockstore)
            .await
            .expect("unable to query node");

        // If this is a file, also track all the blocks we just deleted
        if let Some(PrivateNode::File(file)) = node {
            let cids = file
                .get_cids(&fs.forest, &self.metadata_blockstore)
                .await
                .expect("couldnt get cids for file");
            let string_cids: BTreeSet<String> = cids.iter().map(|cid| cid.to_string()).collect();
            self.deleted_block_cids.extend(string_cids);
        }

        fs.rm(&path_segments, &self.metadata_blockstore)
            .await
            .expect("could not rm");

        log!(
            "tomb-wasm: mount/rm/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );
        self.dirty = true;
        self.sync().await.expect("could not sync");

        // Ok
        Ok(())
    }

    // TODO: migrate betwen mounts

    // TODO: Attaching approved keys to the metadata push
    /// Share with
    /// # Arguments
    /// * bucket_key_id - The id of the bucket key to share with
    /// # Returns
    /// Promise<void> in js speak
    /// # Errors
    /// * `could not read bucket key` - If the bucket key cannot be read (such as if it does not exist, or does not belong to the bucket)
    /// * `Bucket is locked` - If the bucket is locked
    /// * `could not share with` - If the share fails
    #[wasm_bindgen(js_name = shareWith)]
    pub async fn share_with(&mut self, bucket_key_id: String) -> TombResult<()> {
        log!(
            "tomb-wasm: mount/share_with/{}/{}",
            self.bucket.id.to_string(),
            bucket_key_id.clone()
        );
        let bucket_id = self.bucket.id;
        let bucket_key_id = uuid::Uuid::parse_str(&bucket_key_id).expect("Invalid bucket_key UUID");

        let bucket_key = BucketKey::read(bucket_id, bucket_key_id, &mut self.client)
            .await
            .expect("could not read bucket key");

        let recipient_key = bucket_key.pem;
        log!(
            "tomb-wasm: mount/share_with/{} - importing key",
            recipient_key.clone()
        );
        let recipient_key = EcPublicEncryptionKey::import(recipient_key.as_bytes())
            .await
            .expect("could not import key");

        if self.locked() {
            panic!("Bucket is locked");
        };

        self.fs_metadata
            .as_mut()
            .unwrap()
            .share_with(&recipient_key, &self.metadata_blockstore)
            .await
            .expect("could not share with");

        // Mark as dirty so fs is saved with new key info
        self.dirty = true;

        log!(
            "tomb-wasm: mount/share_with/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );

        self.sync().await.expect("could not sync");

        // Ok
        Ok(())
    }

    /// Return boolean indiciating whether or not the currently mounted bucket is snapshotted
    /// # Returns
    /// A boolean
    /// # Errors
    /// * "missing metadata" - If the metadata is missing
    #[wasm_bindgen(js_name = hasSnapshot)]
    pub fn has_snapshot(&self) -> bool {
        log!(
            "tomb-wasm: mount/is_snapshotted/{}",
            self.bucket.id.to_string()
        );
        let metadata = self.metadata.as_ref().expect("missing metadata");
        metadata.snapshot_id.is_some()
    }

    /// Snapshot a mounted bucket
    /// # Returns
    /// A Promise<void> in js speak
    /// # Errors
    /// * "missing metadata" - If the metadata is missing
    /// * "could not snapshot" - If the snapshot fails
    #[wasm_bindgen(js_name = snapshot)]
    pub async fn snapshot(&mut self) -> TombResult<String> {
        log!("tomb-wasm: mount/snapshot/{}", self.bucket.id.to_string());
        let metadata = self.metadata.as_mut().ok_or_else(|| {
            TombWasmError("no metadata associated with mount to snapshot".to_string())
        })?;

        let snapshot_id = metadata
            .snapshot(&mut self.client)
            .await
            .map_err(|err| TombWasmError(format!("unable to take a snapshot: {err}")))?;

        metadata.snapshot_id = Some(snapshot_id);
        self.metadata = Some(metadata.to_owned());

        Ok(snapshot_id.to_string())
    }

    /// Restore a mounted bucket
    /// # Arguments
    /// * `wasm_snapshot` - The snapshot to restore from
    /// # Returns
    /// A Promise<void> in js speak. Should update the mount to the version of the snapshot
    pub async fn restore(&mut self, wasm_snapshot: WasmSnapshot) -> TombResult<()> {
        log!(
            "tomb-wasm: mount/restore/{}/{}",
            self.bucket.id.to_string(),
            wasm_snapshot.id()
        );
        let snapshot = Snapshot::from(wasm_snapshot);
        snapshot
            .restore(&mut self.client)
            .await
            .expect("could not restore snapshot");

        Ok(())
    }
}
