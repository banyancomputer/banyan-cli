mod local;
mod omni;
use crate::prelude::api::requests::core::buckets::metadata::push::PushMetadata;
// mod sync;
// mod error;
// pub(crate) use error::SyncError;
#[allow(unused_imports)]
// pub use sync::{determine_sync_state, sync_bucket, SyncState`};
// pub(crate) use error::SyncError;`
use crate::{
    api::{
        client::Client,
        error::ApiError,
        models::{
            bucket::{Bucket, BucketType, StorageClass},
            metadata::Metadata,
            storage_ticket::StorageTicket,
        },
        requests::staging::upload::content::UploadContent,
    },
    blockstore::{BanyanApiBlockStore, CarV2MemoryBlockStore, RootedBlockStore},
    filesystem::{FilesystemError, FsMetadata},
    native::{configuration::globalconfig::GlobalConfig, operations::restore, NativeError},
};
use colored::Colorize;
use futures_util::StreamExt;
pub use local::LocalBucket;
pub use omni::OmniBucket;
use std::{collections::BTreeSet, fmt::Display};
use tokio::io::AsyncWriteExt;
use tomb_crypt::prelude::{PrivateKey, PublicKey};
use wnfs::{common::BlockStore, libipld::Ipld};

/// Sync State
#[derive(Debug, Clone, PartialEq)]
pub enum SyncState {
    /// Initial / Default state
    Unknown,
    /// There is no remote correlate
    Unpublished,
    /// There is no local correlate
    Unlocalized,
    /// Local bucket is behind the remote
    Behind,
    /// Local and remote are congruent
    MetadataSynced,
    /// Local and remote are congruent
    AllSynced,
    /// Local bucket is ahead of the remote
    Ahead,
}

impl Display for SyncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let description = match self {
            SyncState::Unknown => "Unknown".red(),
            SyncState::Unpublished => "Drive does not exist remotely".red(),
            SyncState::Unlocalized => "Drive does not exist locally".red(),
            SyncState::Behind => "Drive is behind remote".red(),
            SyncState::MetadataSynced => "Metadata Synced; File System not reconstructed".blue(),
            SyncState::AllSynced => "Drive is in sync with remote".green(),
            SyncState::Ahead => "Drive is ahead of remote".red(),
        };

        f.write_fmt(format_args!("{}", description))
    }
}

impl OmniBucket {
    /// Determine the Sync State of an omni bucket
    pub async fn determine_sync_state(&mut self) -> Result<(), NativeError> {
        let bucket_id = match self.get_id() {
            Ok(bucket_id) => bucket_id,
            Err(err) => {
                info!("err: {}", err);
                self.sync_state = SyncState::Unpublished;
                return Ok(());
            }
        };

        // Grab the current remote Metadata, or return Unpublished if that operation fails
        let mut client = GlobalConfig::from_disk().await?.get_client().await?;
        let Ok(current_remote) = Metadata::read_current(bucket_id, &mut client).await else {
            self.sync_state = SyncState::Unpublished;
            return Ok(());
        };
        // Grab the local bucket, or return Unlocalized if unavailable
        if let Ok(local) = self.get_local() {
            let local_metadata_cid = local.metadata.get_root().await.map(|cid| cid.to_string());
            let local_content_cid = local.content.get_root().await.map(|cid| cid.to_string());
            // If the metadata root CIDs match
            if local_metadata_cid == Some(current_remote.metadata_cid) {
                // If the block is also persisted locally in content
                if local_content_cid == Some(current_remote.root_cid) {
                    self.sync_state = SyncState::AllSynced
                } else {
                    self.sync_state = SyncState::MetadataSynced;
                }
                Ok(())
            } else {
                let all_metadatas = Metadata::read_all(bucket_id, &mut client).await?;
                // If the current Metadata id exists in the list of remotely persisted ones
                if all_metadatas
                    .iter()
                    .any(|metadata| Some(metadata.metadata_cid.clone()) == local_metadata_cid)
                {
                    self.sync_state = SyncState::Behind;
                    Ok(())
                } else {
                    self.sync_state = SyncState::Ahead;
                    Ok(())
                }
            }
        } else {
            self.sync_state = SyncState::Unlocalized;
            Ok(())
        }
    }

    /// Sync
    #[allow(unused)]
    pub async fn sync_bucket(&mut self) -> Result<String, NativeError> {
        let mut global = GlobalConfig::from_disk().await?;
        let mut client = global.get_client().await?;
        match &self.sync_state {
            // Download the Bucket
            SyncState::Unlocalized | SyncState::Behind => {
                let current = Metadata::read_current(self.get_id()?, &mut client).await?;
                let mut byte_stream = current.pull(&mut client).await?;

                self.get_or_init_origin().await.ok();

                let mut buffer = <Vec<u8>>::new();
                // Write every chunk to it
                while let Some(chunk) = byte_stream.next().await {
                    tokio::io::copy(&mut chunk.map_err(ApiError::http)?.as_ref(), &mut buffer)
                        .await?;
                }
                // Attempt to create a CARv2 BlockStore from the data
                let metadata = CarV2MemoryBlockStore::try_from(buffer)?;
                // Grab the metadata file
                let mut metadata_file =
                    tokio::fs::File::create(&self.get_local()?.metadata.path).await?;
                metadata_file
                    .write_all(&(metadata.get_data().await))
                    .await?;
                // Write that data out to the metadatas

                info!("{}", "<< METADATA RECONSTRUCTED >>".green());
                self.sync_state = SyncState::MetadataSynced;
                Ok(format!(
                    "{}",
                    "<< DATA STILL NOT DOWNLOADED; SYNC AGAIN >>".blue()
                ))
            }
            // Upload the Bucket
            SyncState::Unpublished | SyncState::Ahead => {
                let mut local = self.get_local()?;
                let wrapping_key = global.wrapping_key().await?;
                let fs = local.unlock_fs(&wrapping_key).await?;

                // If there is still no ID, that means the remote Bucket was never created
                if self.get_id().is_err() {
                    let public_key = wrapping_key.public_key()?;
                    let pem = String::from_utf8(public_key.export().await?)?;
                    let (remote, _) = Bucket::create(
                        local.name.clone(),
                        pem,
                        BucketType::Interactive,
                        StorageClass::Hot,
                        &mut client,
                    )
                    .await?;

                    self.set_remote(remote.clone());
                    local.remote_id = Some(remote.id);
                    global.update_config(&local)?;
                    self.set_local(local.clone());
                }

                // Extract variables or error
                let bucket_id = self.get_id()?;
                let local_content_cid = local
                    .content
                    .get_root()
                    .await
                    .ok_or(FilesystemError::missing_metadata("root cid"))?;
                let local_metadata_cid = local
                    .metadata
                    .get_root()
                    .await
                    .ok_or(FilesystemError::missing_metadata("metdata cid"))?;
                let delta = local.content.get_delta()?;

                // Push the metadata
                let (metadata, host, authorization) = Metadata::push(
                    PushMetadata {
                        bucket_id,
                        expected_data_size: delta.data_size().await,
                        root_cid: local_content_cid.to_string(),
                        metadata_cid: local_metadata_cid.to_string(),
                        previous_cid: local.previous_cid.map(|cid| cid.to_string()),
                        valid_keys: fs.share_manager.public_fingerprints(),
                        deleted_block_cids: local
                            .deleted_block_cids
                            .clone()
                            .iter()
                            .map(|v| v.to_string())
                            .collect(),
                        metadata_stream: tokio::fs::File::open(&local.metadata.path).await?.into(),
                    },
                    &mut client,
                )
                .await?;

                // Empty the list of deleted blocks, now that it's the server's problem
                local.deleted_block_cids = BTreeSet::new();

                if host.is_none() && authorization.is_none() {
                    local.storage_ticket = None;
                }

                info!("Uploading your new data now...");

                let upload_result = match (host, authorization) {
                    // New storage ticket
                    (Some(host), Some(authorization)) => {
                        // Update the storage ticket locally and create grant
                        let storage_ticket = StorageTicket {
                            host,
                            authorization,
                        };
                        storage_ticket.create_grant(&mut client).await?;
                        local.storage_ticket = Some(storage_ticket.clone());
                        local
                            .content
                            .upload(storage_ticket.host, metadata.id, &mut client)
                            .await
                    }
                    // Already granted, still upload
                    (Some(host), None) => {
                        local.content.upload(host, metadata.id, &mut client).await
                    }
                    // No uploading required
                    _ => {
                        global.update_config(&local)?;
                        self.set_local(local);
                        return Ok("METADATA PUSHED; NO CONTENT PUSH NEEDED".to_string());
                    }
                };

                global.update_config(&local)?;
                self.set_local(local);

                match upload_result {
                    // Upload succeeded
                    Ok(()) => {
                        self.sync_state = SyncState::AllSynced;
                        Metadata::read_current(bucket_id, &mut client)
                            .await
                            .map(|new_metadata| {
                                format!(
                                    "{}\n{}",
                                    "<< SUCCESSFULLY UPLOADED METADATA & CONTENT >>".green(),
                                    new_metadata
                                )
                            })
                            .map_err(NativeError::api)
                    }
                    // Upload failed
                    Err(_) => Ok(format!(
                        "{}\n{}\n{}\n",
                        "<< FAILED TO PUSH CONTENT >>".red(),
                        "<< SUCCESSFULLY PUSHED PENDING METADATA >>".green(),
                        metadata
                    )),
                }
            }
            // Reconstruct the Bucket locally
            SyncState::MetadataSynced => {
                let local = self.get_local()?;
                let api_blockstore_client = client.clone();
                let mut api_blockstore = BanyanApiBlockStore::from(api_blockstore_client);
                let metadata_root_cid = local
                    .metadata
                    .get_root()
                    .await
                    .ok_or(FilesystemError::missing_metadata("root cid"))?;
                let mut cids = BTreeSet::new();
                cids.insert(metadata_root_cid);
                api_blockstore.find_cids(cids).await?;
                // If getting a block is an error
                if api_blockstore.get_block(&metadata_root_cid).await.is_err() {
                    // Grab storage host
                    let storage_host = local
                        .clone()
                        .storage_ticket
                        .map(|ticket| ticket.host)
                        .ok_or(NativeError::custom_error(
                            "unable to determine storage host",
                        ))?;
                    // Get authorization
                    let authorization = self.get_remote()?.get_grants_token(&mut client).await?;
                    // Create a grant for this Client so that future BlockStore calls will succeed
                    let storage_ticket = StorageTicket {
                        host: storage_host,
                        authorization,
                    };
                    storage_ticket.create_grant(&mut client).await?;
                }

                // Open the FileSystem
                let fs = FsMetadata::unlock(&global.wrapping_key().await?, &local.metadata).await?;
                // Reconstruct the data on disk
                let restoration_result = restore::pipeline(self.clone()).await;
                // If we succeed at reconstructing
                if restoration_result.is_ok() {
                    // Save the metadata in the content store as well
                    let metadata_cid = local.metadata.get_root().await.unwrap();
                    let ipld = local
                        .metadata
                        .get_deserializable::<Ipld>(&metadata_cid)
                        .await
                        .map_err(Box::from)?;
                    let content_cid = local
                        .content
                        .put_serializable(&ipld)
                        .await
                        .map_err(Box::from)?;
                    local.content.set_root(&content_cid);
                    assert_eq!(metadata_cid, content_cid);
                    // We're now all synced up
                    self.sync_state = SyncState::AllSynced;
                }

                info!("{self}");
                restoration_result
            }
            SyncState::AllSynced => Ok(format!(
                "{}",
                "This Bucket data is already synced :)".green()
            )),
            SyncState::Unknown => {
                self.determine_sync_state().await?;
                Ok(format!(
                    "{}",
                    format!("<< SYNC STATE UPDATED TO {:?} >>", self.sync_state).blue()
                ))
            }
        }
    }
}
