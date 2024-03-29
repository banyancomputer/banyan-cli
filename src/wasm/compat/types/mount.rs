use futures_util::StreamExt;
use js_sys::{Array, ArrayBuffer, Uint8Array};
use std::collections::BTreeSet;
use std::convert::TryFrom;
use std::io::Cursor;
use tomb_crypt::prelude::{EcEncryptionKey, EcPublicEncryptionKey, PrivateKey, PublicKey};
use tracing::info;
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};
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
    filesystem::FsMetadata,
    prelude::{
        api::requests::core::buckets::metadata::push::PushMetadata, blockstore::DoubleSplitStore,
    },
    wasm::{
        to_wasm_error_with_msg, TombResult, TombWasmError, WasmBucket, WasmBucketMetadata,
        WasmFsMetadataEntry, WasmSharedFile, WasmSnapshot,
    },
};

/// Mount point for a Bucket in WASM
///
/// Enables to call Fs methods on a Bucket, pulling metadata from a remote
#[derive(Debug, Clone)]
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

    /// Previous root CID of the Metadata BlockStore
    previous_cid: Option<String>,

    metadata_blockstore: BlockStore,
    content_blockstore: BlockStore,
}

impl WasmMount {
    /// Initialize a new Wasm callable mount with metadata for a bucket and a client
    pub async fn new(
        wasm_bucket: WasmBucket,
        private_pem: String,
        client: &Client,
    ) -> Result<Self, TombWasmError> {
        info!("new()/{}", wasm_bucket.id());

        let bucket = Bucket::from(wasm_bucket.clone());
        info!("new()/{} - creating blockstores", wasm_bucket.id());
        let metadata_blockstore =
            BlockStore::new().map_err(to_wasm_error_with_msg("create blockstore"))?;
        let content_blockstore =
            BlockStore::new().map_err(to_wasm_error_with_msg("create blockstore"))?;
        info!("new()/{} - creating fs metadata", wasm_bucket.id());
        let key = EcEncryptionKey::import(private_pem.as_bytes())
            .await
            .map_err(to_wasm_error_with_msg("import private key pem"))?;
        let fs_metadata = FsMetadata::init(&key)
            .await
            .map_err(to_wasm_error_with_msg("init FsMetadata"))?;
        info!("new()/{} - saving fs metadata", wasm_bucket.id());
        let mut mount = Self {
            client: client.to_owned(),
            bucket,
            metadata: None,
            fs_metadata: Some(fs_metadata),

            locked: false,
            dirty: true,
            append: false,

            deleted_block_cids: BTreeSet::new(),
            metadata_blockstore,
            content_blockstore,
            previous_cid: None,
        };

        info!("new()/{} - syncing", wasm_bucket.id());
        mount.sync().await?;
        // Ok
        Ok(mount)
    }

    /// Initialize a new Wasm callable mount with metadata for a bucket and a client
    pub async fn pull(wasm_bucket: WasmBucket, client: &mut Client) -> Result<Self, TombWasmError> {
        info!("pull()/{}", wasm_bucket.id());
        // Get the underlying bucket
        let bucket = Bucket::from(wasm_bucket.clone());

        // Get the metadata associated with the bucket
        let metadata = Metadata::read_current(bucket.id, client)
            .await
            .map_err(to_wasm_error_with_msg("read metadata"))?;

        let metadata_cid = metadata.metadata_cid.clone();
        info!(
            "pull()/{} - pulling metadata at version {}",
            wasm_bucket.id(),
            metadata_cid
        );
        // Pull the Fs metadata on the matching entry
        let mut stream = metadata
            .pull(client)
            .await
            .map_err(to_wasm_error_with_msg("pull metadata"))?;
        info!("pull()/{} - reading metadata stream", wasm_bucket.id());
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.map_err(to_wasm_error_with_msg("chunk from stream"))?);
        }
        info!("pull()/{} - creating metadata blockstore", wasm_bucket.id());
        let metadata_blockstore =
            BlockStore::try_from(data).map_err(to_wasm_error_with_msg("metadata to blockstore"))?;
        let content_blockstore =
            BlockStore::new().map_err(to_wasm_error_with_msg("create blockstore"))?;

        info!("pull()/{} - pulled", wasm_bucket.id());

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
            previous_cid: Some(metadata_cid),
            fs_metadata: None,
        })
    }

    /// Refresh the current fs_metadata with the remote
    pub async fn refresh(&mut self, key: &EcEncryptionKey) -> Result<(), TombWasmError> {
        info!(bucket_id = ?self.bucket.id, "refresh");

        let bucket_id = self.bucket.id;

        // Get the metadata associated with the bucket
        let metadata = Metadata::read_current(bucket_id, &mut self.client)
            .await
            .map_err(to_wasm_error_with_msg("read current metadata"))?;

        let metadata_cid = metadata.metadata_cid.clone();
        info!(
            "refresh()/{} - pulling metadata at version {}",
            self.bucket.id.to_string(),
            metadata_cid
        );

        // Pull the Fs metadata on the matching entry
        let mut stream = metadata
            .pull(&mut self.client)
            .await
            .map_err(to_wasm_error_with_msg("refresh metadata"))?;

        info!(
            "refresh()/{} - reading metadata stream",
            self.bucket.id.to_string()
        );

        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.map_err(to_wasm_error_with_msg("chunk from stream"))?);
        }

        info!(
            "refresh()/{} - creating metadata blockstore",
            self.bucket.id.to_string()
        );

        let metadata_blockstore =
            BlockStore::try_from(data).map_err(to_wasm_error_with_msg("metadata to blockstore"))?;
        let content_blockstore =
            BlockStore::new().map_err(to_wasm_error_with_msg("create blockstore"))?;

        // does self.bucket need to be updated?
        self.metadata = Some(metadata.to_owned());
        self.metadata_blockstore = metadata_blockstore;
        self.content_blockstore = content_blockstore;
        self.locked = true;
        self.dirty = false;
        self.append = false;
        self.fs_metadata = None;
        self.previous_cid = Some(metadata_cid);

        info!("refresh()/{} - pulled", self.bucket.id.to_string());
        self.unlock(key).await?;
        info!("refresh()/{} - unlocked", self.bucket.id.to_string());

        Ok(())
    }

    /// Sync the current fs_metadata with the remote
    pub async fn sync(&mut self) -> Result<(), TombWasmError> {
        info!("sync()/{}", self.bucket.id.to_string());
        // Check if the bucket is locked
        if self.locked() {
            info!("sync()/{} - bucket is locked", self.bucket.id.to_string());
            panic!("Bucket is locked");
        };
        info!(
            "sync()/{} - saving changes; dirty: {}",
            self.bucket.id.to_string(),
            self.dirty()
        );

        if self.dirty() {
            info!(
                "sync()/{} - saving changes to fs",
                self.bucket.id.to_string()
            );
            let _ = self
                .fs_metadata
                .as_mut()
                .ok_or(TombWasmError::new("missing FsMetadata"))?
                .save(&self.metadata_blockstore, &self.content_blockstore)
                .await;
        } else {
            info!("sync()/{} - no changes to fs", self.bucket.id.to_string());
        }

        info!("sync()/{} - pushing changes", self.bucket.id.to_string());

        let root_cid = self
            .content_blockstore
            .get_root()
            .ok_or(TombWasmError::new("get root cid"))?;
        let metadata_cid = self
            .metadata_blockstore
            .get_root()
            .ok_or(TombWasmError::new("get metadata cid"))?;
        info!(
            "sync()/{} - pushing metadata at version {}",
            self.bucket.id.to_string(),
            metadata_cid.to_string()
        );
        info!(
            "sync()/{} - pushing root at version {}",
            self.bucket.id, root_cid,
        );
        // Assume that the metadata is always at least as big as the content
        let mut data_size = 0;
        if self.append {
            data_size = self.content_blockstore.data_size();
        }
        info!(
            "sync()/{} - metadata cid {} ; content size difference {}",
            self.bucket.id.to_string(),
            metadata_cid.to_string(),
            data_size
        );
        let (metadata, host, authorization) = Metadata::push(
            PushMetadata {
                bucket_id: self.bucket.id,
                expected_data_size: data_size,
                root_cid: root_cid.to_string(),
                metadata_cid: metadata_cid.to_string(),
                previous_cid: self.previous_cid.clone(),
                valid_keys: self
                    .fs_metadata
                    .as_ref()
                    .ok_or(TombWasmError::new("missing FsMetadata"))?
                    .share_manager
                    .public_fingerprints(),
                deleted_block_cids: self.deleted_block_cids.clone(),
                metadata_stream: Cursor::new(self.metadata_blockstore.get_data()),
            },
            &mut self.client,
        )
        .await
        .map_err(to_wasm_error_with_msg("push metadata"))?;

        assert_eq!(metadata.root_cid, root_cid.to_string());
        assert_eq!(metadata.metadata_cid, metadata_cid.to_string());
        let metadata_id = metadata.id;
        self.metadata = Some(metadata);
        self.previous_cid = Some(metadata_cid.to_string());

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
                .map_err(to_wasm_error_with_msg("register storage grant"))?;

                // Then perform upload
                self.content_blockstore
                    .upload(host, metadata_id, &mut self.client)
                    .await
                    .map_err(to_wasm_error_with_msg("created grant; failed upload"))?;
            }
            // Already granted, still upload
            (Some(host), None) => {
                self.content_blockstore
                    .upload(host, metadata_id, &mut self.client)
                    .await
                    .map_err(to_wasm_error_with_msg("no grant; failed upload"))?;
            }
            // No uploading required
            _ => {
                info!("sync()/ - no need to push content");
            }
        }

        self.dirty = false;
        self.append = false;

        info!("sync()/{} - synced", self.bucket.id.to_string());

        Ok(())
    }

    /// Unlock the current fs_metadata
    pub async fn unlock(&mut self, key: &EcEncryptionKey) -> Result<(), TombWasmError> {
        info!("unlock()/{}", self.bucket.id);

        // Check if the bucket is already unlocked
        if !self.locked() {
            return Ok(());
        }

        info!("unlock()/{} - unlocking", self.bucket.id,);

        info!("unlock()/{} - checking versioning", self.bucket.id,);
        let Some(metadata_cid) = self.metadata_blockstore.get_root() else {
            return Err(TombWasmError::new("unable to retrieve metadata CID"));
        };
        // Default to the metadata cid if its not present
        // Remember this is the CID of the IPLD, which will be the same in both cases.
        // TODO change this if we stop using the same CIDs in both.
        let root_cid = self.content_blockstore.get_root().unwrap_or(metadata_cid);

        let metadata = self
            .metadata
            .as_ref()
            .ok_or(TombWasmError::new("missing FsMetadata"))?;

        assert_eq!(metadata_cid.to_string(), metadata.metadata_cid);
        assert_eq!(root_cid.to_string(), metadata.root_cid);

        // Now try unlocking the metadata
        let fs_metadata = FsMetadata::unlock(key, &self.metadata_blockstore)
            .await
            .map_err(to_wasm_error_with_msg("unlock FsMetadata"))?;

        info!("unlock()/{} - unlocked", self.bucket.id,);

        self.locked = false;
        self.fs_metadata = Some(fs_metadata);

        Ok(())
    }

    pub fn content_blockstore(&self) -> BlockStore {
        self.content_blockstore.clone()
    }

    pub fn metadata_blockstore(&self) -> BlockStore {
        self.metadata_blockstore.clone()
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

    /// Returns the Bucket behind the mount
    pub fn bucket(&self) -> WasmBucket {
        WasmBucket::from(self.bucket.clone())
    }

    /// Returns the Metadata for the bucket
    pub fn metadata(&self) -> TombResult<WasmBucketMetadata> {
        let metadata = self
            .metadata
            .as_ref()
            .ok_or(TombWasmError::new("missing FsMetadata"))?;
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
            .map(|s| s.as_string().ok_or(TombWasmError::new("JsValue as string")))
            .collect::<Result<Vec<String>, TombWasmError>>()?;

        info!(
            "ls()/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            return Err(
                TombWasmError::new("unable to list directory contents of a locked bucket").into(),
            );
        };

        info!(
            "ls()/{}/{} - getting entries",
            self.bucket.id,
            &path_segments.join("/")
        );

        // Get the entries
        let fs_metadata_entries = self
            .fs_metadata
            .as_ref()
            .ok_or(TombWasmError::new("missing FsMetadata"))?
            .ls(&path_segments, &self.metadata_blockstore)
            .await
            .map_err(to_wasm_error_with_msg("list directory entries"))?;

        info!("ls()/{} - mapping entries", self.bucket.id);

        // Map the entries back to JsValues
        fs_metadata_entries
            .iter()
            .map(|entry| {
                let wasm_fs_metadata_entry = WasmFsMetadataEntry::from(entry.clone());
                JsValue::try_from(wasm_fs_metadata_entry).map_err(|err| {
                    TombWasmError::new(&format!(
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
            .map(|s| s.as_string().ok_or(TombWasmError::new("JsValue as string")))
            .collect::<Result<Vec<String>, TombWasmError>>()?;

        info!(
            "mkdir()/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        info!(
            "mkdir()/{}/{} - mkdir",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );
        self.fs_metadata
            .as_mut()
            .ok_or(TombWasmError::new("missing FsMetadata"))?
            .mkdir(&path_segments, &self.metadata_blockstore)
            .await
            .map_err(to_wasm_error_with_msg("mkdir"))?;

        info!(
            "mkdir()/{}/{} - dirty, syncing changes",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );
        self.dirty = true;
        self.sync().await?;

        // Ok
        Ok(())
    }

    /// Refreshes the bucket to ensure its up to date against what the server is aware of
    pub async fn remount(&mut self, encryption_key_pem: String) -> TombResult<()> {
        info!(bucket_id = ?self.bucket.id, "remount");

        let key = EcEncryptionKey::import(encryption_key_pem.as_bytes())
            .await
            .map_err(|e| TombWasmError::new(&e.to_string()))?;

        self.refresh(&key).await?;

        info!(bucket_id = ?self.bucket.id, "remount complete");

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
            .map(|s| s.as_string().ok_or(TombWasmError::new("JsValue as string")))
            .collect::<Result<Vec<String>, TombWasmError>>()?;

        info!(
            "add()/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        let content = Uint8Array::new(&content_buffer).to_vec();

        self.fs_metadata
            .as_mut()
            .ok_or(TombWasmError::new("missing FsMetadata"))?
            .write(
                &path_segments,
                &self.metadata_blockstore,
                &self.content_blockstore,
                content,
            )
            .await
            .map_err(to_wasm_error_with_msg("fs add"))?;
        info!(
            "add()/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );
        self.dirty = true;
        self.append = true;

        self.sync().await?;

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
            .map(|s| s.as_string().ok_or(TombWasmError::new("JsValue as string")))
            .collect::<Result<Vec<String>, TombWasmError>>()?;

        info!(
            "read_bytes()/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        let api_blockstore_client = self.client.clone();
        let api_blockstore = BanyanApiBlockStore::from(api_blockstore_client);

        let fs = self
            .fs_metadata
            .as_mut()
            .ok_or(TombWasmError::new("missing FsMetadata"))?;

        let node = fs
            .get_node(&path_segments, &self.metadata_blockstore)
            .await
            .map_err(to_wasm_error_with_msg("access FsMetadata"))?
            .ok_or(TombWasmError::new("no node at path"))?;

        if let PrivateNode::File(file) = node {
            let cids = file
                .get_cids(&fs.forest, &self.metadata_blockstore)
                .await
                .map_err(|_| TombWasmError::new("retrieve CIDs"))?;
            api_blockstore
                .find_cids(cids)
                .await
                .map_err(to_wasm_error_with_msg("find_cids"))?;
        }

        info!("read_bytes() running fs.read @ {:?}", path_segments);

        // Attempt to fetch from local first, remote second
        let split_store = DoubleSplitStore::new(&self.content_blockstore, &api_blockstore);
        let vec = fs
            .read(&path_segments, &self.metadata_blockstore, &split_store)
            .await
            .map_err(to_wasm_error_with_msg("read node bytes"))?;

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
            .map(|s| s.as_string().ok_or(TombWasmError::new("JsValue as string")))
            .collect::<Result<Vec<String>, TombWasmError>>()?;
        let to_path_segments = to_path_segments
            .iter()
            .map(|s| s.as_string().ok_or(TombWasmError::new("JsValue as string")))
            .collect::<Result<Vec<String>, TombWasmError>>()?;

        info!(
            "mv()/{}/{} => {}",
            self.bucket.id.to_string(),
            &from_path_segments.join("/"),
            &to_path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        self.fs_metadata
            .as_mut()
            .ok_or(TombWasmError::new("missing FsMetadata"))?
            .mv(
                &from_path_segments,
                &to_path_segments,
                &self.metadata_blockstore,
                &self.content_blockstore,
            )
            .await
            .map_err(to_wasm_error_with_msg("fs mv"))?;

        info!(
            "mv()/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );

        self.dirty = true;
        // In order to keep sharing working, we need to append the content blockstore on move.
        // Not ideal but it works.
        self.append = true;

        self.sync().await?;

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
            .map(|s| s.as_string().ok_or(TombWasmError::new("JsValue as string")))
            .collect::<Result<Vec<String>, TombWasmError>>()?;

        info!(
            "rm()/{}/{}",
            self.bucket.id.to_string(),
            path_segments.join("/")
        );

        if self.locked() {
            panic!("Bucket is locked");
        };

        let fs = self
            .fs_metadata
            .as_mut()
            .ok_or(TombWasmError::new("missing FsMetadata"))?;

        let node = fs
            .get_node(&path_segments, &self.metadata_blockstore)
            .await
            .map_err(to_wasm_error_with_msg("query node"))?;

        // If this is a file, also track all the blocks we just deleted
        if let Some(PrivateNode::File(file)) = node {
            let cids = file
                .get_cids(&fs.forest, &self.metadata_blockstore)
                .await
                .ok()
                .ok_or(TombWasmError::new("get CIDs"))?;
            let string_cids: BTreeSet<String> = cids.iter().map(|cid| cid.to_string()).collect();
            self.deleted_block_cids.extend(string_cids);
        }

        fs.rm(&path_segments, &self.metadata_blockstore)
            .await
            .map_err(to_wasm_error_with_msg("fs rm"))?;

        info!(
            "rm()/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );
        self.dirty = true;
        self.sync().await?;

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
        info!(
            "share_with/{}/{}",
            self.bucket.id.to_string(),
            bucket_key_id.clone()
        );
        let bucket_id = self.bucket.id;
        let bucket_key_id =
            uuid::Uuid::parse_str(&bucket_key_id).map_err(to_wasm_error_with_msg("parse UUID"))?;

        let bucket_key = BucketKey::read(bucket_id, bucket_key_id, &mut self.client)
            .await
            .map_err(to_wasm_error_with_msg("read drive key"))?;

        let recipient_key = bucket_key.pem;
        info!("share_with/{} - importing key", recipient_key.clone());
        let recipient_key = EcPublicEncryptionKey::import(recipient_key.as_bytes())
            .await
            .map_err(to_wasm_error_with_msg("import recipient key"))?;

        if self.locked() {
            panic!("Bucket is locked");
        };

        self.fs_metadata
            .as_mut()
            .ok_or(TombWasmError::new("missing FsMetadata"))?
            .share_with(&recipient_key, &self.metadata_blockstore)
            .await
            .map_err(to_wasm_error_with_msg("fs share_with"))?;

        // Mark as dirty so fs is saved with new key info
        self.dirty = true;

        info!(
            "share_with/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );

        self.sync().await?;

        // Ok
        Ok(())
    }

    /// Share a file snapshot
    #[wasm_bindgen(js_name = shareFile)]
    pub async fn share_file(&mut self, path_segments: Array) -> TombResult<String> {
        info!(
            "share_file()/{}/{}",
            self.bucket.id.to_string(),
            &path_segments.join("/")
        );

        // Read the array as a Vec<String>
        let path_segments = path_segments
            .iter()
            .map(|s| s.as_string().ok_or(TombWasmError::new("JsValue as string")))
            .collect::<Result<Vec<String>, TombWasmError>>()?;

        if self.locked() {
            return Err(TombWasmError::new("unable to share a file from a locked bucket").into());
        };

        let shared_file = self
            .fs_metadata
            .as_mut()
            .ok_or(TombWasmError::new("missing FsMetadata"))?
            .share_file(
                &path_segments,
                &self.metadata_blockstore,
                &self.content_blockstore,
            )
            .await
            .map_err(to_wasm_error_with_msg("share_file"))?;

        // Mark as dirty so and additional blocks are persisted remotely
        self.dirty = true;
        // Mark as append so the content blockstore is uploaded
        self.append = true;

        info!(
            "share_file/{} - dirty, syncing changes",
            self.bucket.id.to_string()
        );

        self.sync().await?;

        let shared_file = WasmSharedFile(shared_file);
        Ok(shared_file.export_b64_url()?)
    }

    /// Return boolean indiciating whether or not the currently mounted bucket is snapshotted
    /// # Returns
    /// A boolean
    /// # Errors
    /// * "missing metadata" - If the metadata is missing
    #[wasm_bindgen(js_name = hasSnapshot)]
    pub fn has_snapshot(&self) -> bool {
        info!("has_snapshot()/{}", self.bucket.id.to_string());
        let metadata = self
            .metadata
            .as_ref()
            .ok_or(TombWasmError::new("missing FsMetadata"))
            .unwrap();

        metadata.snapshot_id.is_some()
    }

    /// Snapshot a mounted bucket
    /// # Returns
    /// A Promise<void> in js speak
    /// # Errors
    /// * "missing metadata" - If the metadata is missing
    /// * "could not snapshot" - If the snapshot fails
    pub async fn snapshot(&mut self) -> TombResult<String> {
        info!("snapshot()/{}", self.bucket.id.to_string());
        let metadata = self
            .metadata
            .as_mut()
            .ok_or_else(|| TombWasmError::new("no metadata associated with mount to snapshot"))?;

        let fs = self
            .fs_metadata
            .as_mut()
            .ok_or(TombWasmError::new("missing FsMetadata"))?;

        let all_nodes = fs
            .get_all_nodes(&self.metadata_blockstore)
            .await
            .map_err(to_wasm_error_with_msg("get all nodes"))?;

        let mut active_cids = BTreeSet::new();
        for (node, _) in all_nodes {
            if let PrivateNode::File(file) = node {
                let file_cids = file
                    .get_cids(&fs.forest, &self.metadata_blockstore)
                    .await
                    .map_err(|_| TombWasmError::new("get cids"))?;
                active_cids.extend(file_cids);
            }
        }

        let snapshot_id = metadata
            .snapshot(active_cids, &mut self.client)
            .await
            .map_err(to_wasm_error_with_msg("take drive snapshot"))?;

        metadata.snapshot_id = Some(snapshot_id);
        self.metadata = Some(metadata.to_owned());

        Ok(snapshot_id.to_string())
    }

    /// Rename the mounted bucket
    /// # Arguments
    /// * `name` - the new name for the bucket
    /// # Returns
    /// A Promise<void> in js speak. Should also update the internal state of the bucket
    /// on a successful update
    pub async fn rename(&mut self, name: String) -> TombResult<()> {
        info!("rename()/{}/{}", self.bucket.id.to_string(), &name);
        let mut update_bucket = self.bucket.clone();
        update_bucket.name = name;
        update_bucket
            .update(&mut self.client)
            .await
            .map_err(to_wasm_error_with_msg("rename bucket"))?;
        self.bucket = update_bucket;
        Ok(())
    }

    /// Restore a mounted bucket
    /// # Arguments
    /// * `wasm_snapshot` - The snapshot to restore from
    /// # Returns
    /// A Promise<void> in js speak. Should update the mount to the version of the snapshot
    pub async fn restore(&mut self, wasm_snapshot: WasmSnapshot) -> TombResult<()> {
        info!(
            "restore()/{}/{}",
            self.bucket.id.to_string(),
            wasm_snapshot.id()
        );
        let snapshot = Snapshot::from(wasm_snapshot);
        snapshot
            .restore(&mut self.client)
            .await
            .map_err(to_wasm_error_with_msg("restore snapshot"))?;

        Ok(())
    }
}
