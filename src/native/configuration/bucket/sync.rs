use super::OmniBucket;
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
    native::{
        configuration::{globalconfig::GlobalConfig, ConfigurationError},
        operations::{restore, OperationError},
    },
};
use colored::Colorize;
use futures_util::StreamExt;
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

/// Determine the Sync State of an omni bucket
pub async fn determine_sync_state(
    omni: &mut OmniBucket,
    client: &mut Client,
) -> Result<(), ConfigurationError> {
    let bucket_id = match omni.get_id() {
        Ok(bucket_id) => bucket_id,
        Err(err) => {
            info!("err: {}", err);
            omni.sync_state = SyncState::Unpublished;
            return Ok(());
        }
    };

    // Grab the current remote Metadata, or return Unpublished if that operation fails
    let Ok(current_remote) = Metadata::read_current(bucket_id, client).await else {
        omni.sync_state = SyncState::Unpublished;
        return Ok(());
    };
    // Grab the local bucket, or return Unlocalized if unavailable
    if let Ok(local) = omni.get_local() {
        let local_metadata_cid = local.metadata.get_root().map(|cid| cid.to_string());
        let local_content_cid = local.content.get_root().map(|cid| cid.to_string());
        // If the metadata root CIDs match
        if local_metadata_cid == Some(current_remote.metadata_cid) {
            // If the block is also persisted locally in content
            if local_content_cid == Some(current_remote.root_cid) {
                omni.sync_state = SyncState::AllSynced
            } else {
                omni.sync_state = SyncState::MetadataSynced;
            }
            Ok(())
        } else {
            let all_metadatas = Metadata::read_all(bucket_id, client).await?;
            // If the current Metadata id exists in the list of remotely persisted ones
            if all_metadatas
                .iter()
                .any(|metadata| Some(metadata.metadata_cid.clone()) == local_metadata_cid)
            {
                omni.sync_state = SyncState::Behind;
                Ok(())
            } else {
                omni.sync_state = SyncState::Ahead;
                Ok(())
            }
        }
    } else {
        omni.sync_state = SyncState::Unlocalized;
        Ok(())
    }
}

/// Sync
#[allow(unused)]
pub async fn sync_bucket(
    omni: &mut OmniBucket,
    client: &mut Client,
    global: &mut GlobalConfig,
) -> Result<String, ConfigurationError> {
    match &omni.sync_state {
        // Download the Bucket
        SyncState::Unlocalized | SyncState::Behind => {
            let current = Metadata::read_current(omni.get_id()?, client).await?;
            let mut byte_stream = current.pull(client).await?;

            omni.get_or_init_origin(global).await.ok();

            let mut buffer = <Vec<u8>>::new();
            // Write every chunk to it
            while let Some(chunk) = byte_stream.next().await {
                tokio::io::copy(&mut chunk.map_err(ApiError::http)?.as_ref(), &mut buffer).await?;
            }
            // Attempt to create a CARv2 BlockStore from the data
            let metadata = CarV2MemoryBlockStore::try_from(buffer)?;
            // Grab the metadata file
            let mut metadata_file =
                tokio::fs::File::create(&omni.get_local()?.metadata.path).await?;
            metadata_file.write_all(&metadata.get_data()).await?;
            // Write that data out to the metadatas

            info!("{}", "<< METADATA RECONSTRUCTED >>".green());
            omni.sync_state = SyncState::MetadataSynced;
            Ok(format!(
                "{}",
                "<< DATA STILL NOT DOWNLOADED; SYNC AGAIN >>".blue()
            ))
        }
        // Upload the Bucket
        SyncState::Unpublished | SyncState::Ahead => {
            let mut local = omni.get_local()?;
            let wrapping_key = global.wrapping_key().await?;
            let fs = local.unlock_fs(&wrapping_key).await?;

            // If there is still no ID, that means the remote Bucket was never created
            if omni.get_id().is_err() {
                let public_key = wrapping_key.public_key()?;
                let pem = String::from_utf8(public_key.export().await?)?;
                let (remote, _) = Bucket::create(
                    local.name.clone(),
                    pem,
                    BucketType::Interactive,
                    StorageClass::Hot,
                    client,
                )
                .await?;

                omni.set_remote(remote.clone());
                local.remote_id = Some(remote.id);
                omni.set_local(local.clone());
            }

            // Extract variables or error
            let bucket_id = omni.get_id()?;
            let local_content_cid = local
                .content
                .get_root()
                .ok_or(FilesystemError::missing_metadata("root cid"))?;
            let local_metadata_cid = local
                .metadata
                .get_root()
                .ok_or(FilesystemError::missing_metadata("metdata cid"))?;
            let delta = local.content.get_delta()?;

            // Push the metadata
            let (metadata, host, authorization) = Metadata::push(
                bucket_id,
                local_content_cid.to_string(),
                local_metadata_cid.to_string(),
                delta.data_size(),
                fs.share_manager.public_fingerprints(),
                local
                    .deleted_block_cids
                    .clone()
                    .iter()
                    .map(|v| v.to_string())
                    .collect(),
                tokio::fs::File::open(&local.metadata.path).await?.into(),
                client,
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
                    storage_ticket.create_grant(client).await?;
                    local.storage_ticket = Some(storage_ticket.clone());
                    local
                        .content
                        .upload(storage_ticket.host, metadata.id, client)
                        .await
                }
                // Already granted, still upload
                (Some(host), None) => local.content.upload(host, metadata.id, client).await,
                // No uploading required
                _ => {
                    global.update_config(&local)?;
                    omni.set_local(local);
                    return Ok("METADATA PUSHED; NO CONTENT PUSH NEEDED".to_string());
                }
            };

            global.update_config(&local)?;
            omni.set_local(local);

            match upload_result {
                // Upload succeeded
                Ok(()) => {
                    omni.sync_state = SyncState::AllSynced;
                    Metadata::read_current(bucket_id, client)
                        .await
                        .map(|new_metadata| {
                            format!(
                                "{}\n{}",
                                "<< SUCCESSFULLY UPLOADED METADATA & CONTENT >>".green(),
                                new_metadata
                            )
                        })
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
            let local = omni.get_local()?;
            let storage_host = local
                .clone()
                .storage_ticket
                .map(|ticket| ticket.host)
                .unwrap_or(global.endpoints.data.clone());

            let mut api_blockstore_client = client.clone();
            api_blockstore_client
                .with_remote(&storage_host)
                .expect("could not create blockstore client");

            let api_blockstore = BanyanApiBlockStore::from(api_blockstore_client);
            // If getting a block is an error
            if api_blockstore
                .get_block(
                    &local
                        .metadata
                        .get_root()
                        .ok_or(FilesystemError::missing_metadata("root cid"))?,
                )
                .await
                .is_err()
            {
                // Get authorization
                let authorization = omni.get_remote()?.get_grants_token(client).await?;
                // Create a grant for this Client so that future BlockStore calls will succeed
                let storage_ticket = StorageTicket {
                    host: storage_host,
                    authorization,
                };
                storage_ticket.create_grant(client).await?;
            }

            // Open the FileSystem
            let fs = FsMetadata::unlock(&global.wrapping_key().await?, &local.metadata).await?;
            // Reconstruct the data on disk
            let restoration_result = restore::pipeline(fs, omni, client).await;
            // If we succeed at reconstructing
            if restoration_result.is_ok() {
                // Save the metadata in the content store as well
                let metadata_cid = local.metadata.get_root().unwrap();
                let ipld = local
                    .metadata
                    .get_deserializable::<Ipld>(&metadata_cid)
                    .await?;
                let content_cid = local.content.put_serializable(&ipld).await?;
                local.content.set_root(&content_cid);
                assert_eq!(metadata_cid, content_cid);
                // We're now all synced up
                omni.sync_state = SyncState::AllSynced;
            }

            info!("{omni}");
            restoration_result
        }
        SyncState::AllSynced => Ok(format!(
            "{}",
            "This Bucket data is already synced :)".green()
        )),
        SyncState::Unknown => {
            determine_sync_state(omni, client).await?;
            Ok(format!(
                "{}",
                format!("<< SYNC STATE UPDATED TO {:?} >>", omni.sync_state).blue()
            ))
        }
    }
}
