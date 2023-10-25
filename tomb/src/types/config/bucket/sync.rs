use colored::Colorize;
use futures_util::StreamExt;
use std::{
    collections::BTreeSet,
    fmt::Display,
    fs::{create_dir_all, remove_dir_all},
    path::PathBuf,
};
use tokio::io::AsyncWriteExt;
use tomb_common::{
    banyan_api::{
        blockstore::BanyanApiBlockStore, client::Client, error::ClientError,
        models::metadata::Metadata,
    },
    blockstore::{carv2_memory::CarV2MemoryBlockStore, RootedBlockStore},
    metadata::FsMetadata,
};

use crate::{
    pipelines::{error::TombError, reconstruct},
    types::config::globalconfig::GlobalConfig,
};

use super::OmniBucket;

/// Sync State
#[derive(Debug, Clone)]
pub enum SyncState {
    /// There is no remote correlate
    Unpublished,
    /// There is no local correlate
    Unlocalized,
    /// Local bucket is N commits behind the remote
    Behind(usize),
    /// Local and remote are congruent
    MetadataSynced,
    /// Local and remote are congruent
    AllSynced,
    /// Local bucket is N commits ahead of the remote
    Ahead(usize),
}

impl Display for SyncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let description = match self {
            SyncState::Unpublished => "Bucket metadata does not exist remotely".red(),
            SyncState::Unlocalized => "Bucket metadata not exist locally".red(),
            SyncState::Behind(n) => format!("Bucket is {} commits behind remote", n).red(),
            SyncState::MetadataSynced => {
                "Bucket metadata is in sync with remote but has not been reconstructed locally"
                    .blue()
            }
            SyncState::AllSynced => "Bucket is in sync with remote".green(),
            SyncState::Ahead(n) => format!("Bucket is {} commits ahead of remote", n).red(),
        };

        f.write_fmt(format_args!("{}", description))
    }
}

/// Determine the Sync State of an omni bucket
pub async fn determine_sync_state(
    omni: &mut OmniBucket,
    client: &mut Client,
) -> Result<(), TombError> {
    let bucket_id = omni.get_id()?;
    // Grab the current remote Metadata, or return Unpublished if that operation fails
    let Ok(current_remote) = Metadata::read_current(bucket_id, client).await else {
        omni.sync_state = Some(SyncState::Unpublished);
        return Ok(());
    };
    // Grab the local bucket, or return Unlocalized if unavailable
    if let Ok(local) = omni.get_local() {
        let local_root_cid = local.metadata.get_root().map(|cid| cid.to_string());
        // If the metadata root CIDs match
        if local_root_cid == Some(current_remote.root_cid) {
            // If there is actually data in the local origin
            let _tolerance = 100;
            let _expect_data_size = current_remote.data_size;
            // If the data size matches the most recent delta
            // if let Some(delta) = local.content.deltas.last() && let Ok(actual_data_size) = compute_directory_size(&delta.path).map(|v| v as u64) && actual_data_size >= expect_data_size-tolerance && actual_data_size < expect_data_size+tolerance {
            //     omni.sync_state = Some(SyncState::AllSynced);
            // } else {
            // }
            omni.sync_state = Some(SyncState::MetadataSynced);
            Ok(())
        } else {
            let all_metadatas = Metadata::read_all(bucket_id, client).await?;
            // If the current Metadata id exists in the list of remotely persisted ones
            if all_metadatas
                .iter()
                .any(|metadata| Some(metadata.root_cid.clone()) == local_root_cid)
            {
                omni.sync_state = Some(SyncState::Behind(1));
                Ok(())
            } else {
                omni.sync_state = Some(SyncState::Ahead(1));
                Ok(())
            }
        }
    } else {
        omni.sync_state = Some(SyncState::Unlocalized);
        Ok(())
    }
}

/// Sync
pub async fn sync_bucket(
    omni: &mut OmniBucket,
    client: &mut Client,
    global: &mut GlobalConfig,
) -> Result<String, TombError> {
    if omni.sync_state.is_none() {
        println!("{}", "<< SYNC STATE UPDATED >>".blue());
        println!("{:?}", determine_sync_state(omni, client).await);
    }

    match &omni.sync_state {
        // Download the Bucket
        Some(SyncState::Unlocalized) | Some(SyncState::Behind(_)) => {
            let current = Metadata::read_current(omni.get_id()?, client).await?;
            let mut byte_stream = current.pull(client).await?;

            let new_local_origin = PathBuf::from(env!("HOME"))
                .join("tomb")
                .join(omni.get_remote()?.name);
            // Remove existing contents and create a enw directory
            remove_dir_all(&new_local_origin).ok();
            create_dir_all(&new_local_origin)?;

            // Create a new
            omni.set_local({
                let mut value = global
                    .get_or_init_bucket(&omni.get_remote()?.name, &new_local_origin)
                    .await?;
                value.remote_id = Some(omni.get_remote()?.id);
                value
            });

            let mut buffer = <Vec<u8>>::new();
            // Write every chunk to it
            while let Some(chunk) = byte_stream.next().await {
                tokio::io::copy(
                    &mut chunk.map_err(ClientError::http_error)?.as_ref(),
                    &mut buffer,
                )
                .await?;
            }
            // Attempt to create a CARv2 BlockStore from the data
            let metadata = CarV2MemoryBlockStore::try_from(buffer).map_err(|_| {
                TombError::custom_error("Failed to represent metadata download as blockstore")
            })?;
            // Grab the metadata file
            let mut metadata_file =
                tokio::fs::File::create(&omni.get_local()?.metadata.path).await?;
            metadata_file.write_all(&metadata.get_data()).await?;
            // Write that data out to the metadata

            println!("{}", "<< METADATA RECONSTRUCTED >>".green());
            omni.sync_state = Some(SyncState::MetadataSynced);
            Ok(format!(
                "{}",
                "<< DATA STILL NOT DOWNLOADED; SYNC AGAIN >>".blue()
            ))
        }
        // Upload the Bucket
        Some(SyncState::Unpublished | SyncState::Ahead(_)) => {
            let mut local = omni.get_local()?;
            let wrapping_key = global.wrapping_key().await?;
            let fs = local.unlock_fs(&wrapping_key).await?;

            // If we can actually get the arguments
            if let Some(bucket_id) = local.remote_id && let Some(root_cid) = local.metadata.get_root() &&
               let Some(delta) = local.content.deltas.last() {
                // Push the metadata
                let (metadata, storage_ticket) = Metadata::push(
                    bucket_id,
                    root_cid.to_string(),
                    root_cid.to_string(),
                    delta.data_size(),
                    fs.share_manager.public_fingerprints(),
                    local.deleted_blocks.clone().iter().map(|v| v.to_string()).collect(),
                    tokio::fs::File::open(&local.metadata.path).await?,
                    client
                ).await?;
                // Empty the list of deleted blocks, now that it's the server's problem
                local.deleted_blocks = BTreeSet::new();
                // Update storage ticket in the local configurations for future pulls
                local.storage_ticket = storage_ticket.clone();
                global.update_config(&local)?;
                omni.set_local(local.clone());

                // If the storage ticket is valid
                let storage_ticket = storage_ticket.ok_or_else(|| TombError::custom_error("Metadata was pushed but storage sticket was not reccieved"))?;
                // Create storage grant
                storage_ticket
                    .clone()
                    .create_grant(client)
                    .await
                    .map_err(|err| {
                        TombError::custom_error(&format!("unable to register storage ticket: {err}"))
                    })?;

                println!("successfully created the grant; now pushing content from delta: {}", delta.path.display());

                // Push content to the storage provider
                let delta_reader = std::fs::File::open(&delta.path)?;
                let content_len = delta_reader.metadata()?.len();
                let mut hasher = blake3::Hasher::new();
                hasher.update_reader(delta_reader)?;
                let content_hash = hasher.finalize().to_string();
                let delta_reader = tokio::fs::File::open(&delta.path).await?;

                match storage_ticket.upload_content(metadata.id, delta_reader, content_len, content_hash, client).await {
                    // Upload succeeded
                    Ok(_) => {
                        omni.sync_state = Some(SyncState::AllSynced);
                        Metadata::read_current(bucket_id, client).await.map(|new_metadata| format!("{}\n{}", "<< SUCCESSFULLY UPLOADED METADATA & CONTENT >>".green(), new_metadata)).map_err(TombError::client_error)
                    },
                    // Upload failed
                    Err(err) => {
                        println!("err: {}", err);
                        Ok(format!("{}\n{}\n{}\n", "<< FAILED TO PUSH CONTENT >>".red(), "<< SUCCESSFULLY PUSHED PENDING METADATA >>".green(), metadata))
                    },
                }
            } else {
                Err(TombError::custom_error("No metadata to push, or no content deltas"))
            }
        }
        // Reconstruct the Bucket locally
        Some(SyncState::MetadataSynced) => {
            let local = omni.get_local()?;
            let storage_host = local
                .clone()
                .storage_ticket
                .map(|ticket| ticket.host)
                .unwrap_or(global.endpoints.data.clone());

            let mut banyan_api_blockstore_client = client.clone();
            banyan_api_blockstore_client
                .with_remote(&storage_host)
                .expect("could not create blockstore client");

            let banyan_api_blockstore = BanyanApiBlockStore::from(banyan_api_blockstore_client);
            println!("banyan_api_blockstore constructed");

            let fs = FsMetadata::unlock(
                &GlobalConfig::from_disk().await?.wrapping_key().await?,
                &local.metadata,
            )
            .await?;
            let mut store = CarV2MemoryBlockStore::new()?;
            let forest_root = fs.forest.store(&mut store).await?;
            println!("forest_root: {:?}", forest_root);

            // Reconstruct the data on disk
            let reconstruction_result =
                reconstruct::pipeline(global, &local, &banyan_api_blockstore, &local.origin).await;
            if reconstruction_result.is_ok() {
                omni.sync_state = Some(SyncState::AllSynced);
            }
            reconstruction_result
        }
        Some(SyncState::AllSynced) => Ok("already synced".into()),
        None => Err(TombError::custom_error("Unable to determine sync state")),
    }
}
