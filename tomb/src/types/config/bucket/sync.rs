use crate::{
    pipelines::{error::TombError, reconstruct},
    types::config::globalconfig::GlobalConfig,
};
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
        blockstore::BanyanApiBlockStore,
        client::Client,
        error::ClientError,
        models::{metadata::Metadata, storage_ticket::StorageTicket},
        requests::staging::upload::content::UploadContent,
    },
    blockstore::{carv2_memory::CarV2MemoryBlockStore, RootedBlockStore},
};
use wnfs::common::BlockStore;

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
            // TODO determine a reliable way to check if the content is synced too
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

            // Extract variables or error
            let bucket_id = local
                .remote_id
                .ok_or(TombError::custom_error("Bucket has no remote ID"))?;
            let root_cid = local
                .metadata
                .get_root()
                .ok_or(TombError::custom_error("Bucket has no root CID"))?;
            let delta = local.content.get_delta()?;

            // Push the metadata
            let (metadata, host, authorization) = Metadata::push(
                bucket_id,
                root_cid.to_string(),
                root_cid.to_string(),
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

            // If a full storage ticket was returned
            if let Some(host) = host.clone() {
                if let Some(authorization) = authorization {
                    let storage_ticket = StorageTicket {
                        host,
                        authorization,
                    };

                    // Update the storage ticket locally
                    local.storage_ticket = Some(storage_ticket.clone());

                    // Create a grant
                    storage_ticket.create_grant(client).await?;
                }
            }

            // Update global and local configs
            global.update_config(&local)?;
            omni.set_local(local.clone());

            if let Some(host_url) = host {
                // Push content to the storage provider
                match local.content.upload(host_url, metadata.id, client).await {
                    // Upload succeeded
                    Ok(_) => {
                        omni.sync_state = Some(SyncState::AllSynced);
                        Metadata::read_current(bucket_id, client)
                            .await
                            .map(|new_metadata| {
                                format!(
                                    "{}\n{}",
                                    "<< SUCCESSFULLY UPLOADED METADATA & CONTENT >>".green(),
                                    new_metadata
                                )
                            })
                            .map_err(TombError::client_error)
                    }
                    // Upload failed
                    Err(_) => Ok(format!(
                        "{}\n{}\n{}\n",
                        "<< FAILED TO PUSH CONTENT >>".red(),
                        "<< SUCCESSFULLY PUSHED PENDING METADATA >>".green(),
                        metadata
                    )),
                }
            } else {
                Ok("METADATA PUSHED; NO CONTENT PUSH NEEDED".to_string())
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

            // If getting a block is an error
            if banyan_api_blockstore
                .get_block(
                    &local
                        .metadata
                        .get_root()
                        .ok_or(TombError::custom_error("no metadata root_cid"))?,
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

            let local = omni.get_local()?;
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

#[cfg(test)]
#[cfg(feature = "fake")]
pub mod test {
    use crate::{
        pipelines::error::TombError,
        types::config::{bucket::OmniBucket, globalconfig::GlobalConfig},
    };
    use serial_test::serial;
    use std::{collections::BTreeSet, path::PathBuf};
    use tomb_common::{
        banyan_api::{
            blockstore::BanyanApiBlockStore,
            client::Client,
            models::{account::Account, metadata::Metadata, storage_ticket::StorageTicket},
            requests::{
                core::blocks::locate::LocationRequest, staging::upload::content::UploadContent,
            },
        },
        blockstore::{split::DoubleSplitStore, RootedBlockStore},
    };
    use wnfs::libipld::Cid;

    pub async fn authenticated_client() -> Client {
        let mut client = Client::new("http://127.0.0.1:3001", "http://127.0.0.1:3002").unwrap();
        let _ = Account::create_fake(&mut client).await.unwrap();
        client
    }

    // Helper function to set up an environment with a small amount of delta data and push the metadata associated
    pub async fn setup_push_metadata(
        test_name: &str,
    ) -> Result<(Client, OmniBucket, Metadata, StorageTicket), TombError> {
        let origin = PathBuf::from("test").join(test_name);
        let mut client = authenticated_client().await;
        let mut global = GlobalConfig::from_disk().await?;

        let omni = OmniBucket::create(&mut global, &mut client, test_name, &origin).await?;
        let local = omni.get_local()?;
        let mut fs = omni
            .get_local()?
            .unlock_fs(&global.wrapping_key().await?)
            .await?;
        fs.write(
            &["cat.txt".to_string()],
            &local.metadata,
            &local.content,
            b"Example content".to_vec(),
        )
        .await?;
        fs.save(&local.metadata, &local.content).await?;

        let (metadata, host, authorization) = Metadata::push(
            omni.get_id()?,
            local.content.get_root().unwrap().to_string(),
            local.metadata.get_root().unwrap().to_string(),
            local.content.get_delta()?.data_size(),
            fs.share_manager.public_fingerprints(),
            BTreeSet::new(),
            tokio::fs::File::open(&local.metadata.path).await?.into(),
            &mut client,
        )
        .await?;

        let storage_ticket = StorageTicket {
            host: host.unwrap(),
            authorization: authorization.unwrap(),
        };

        Ok((client, omni, metadata, storage_ticket))
    }

    #[tokio::test]
    #[serial]
    async fn create_grant() -> Result<(), TombError> {
        let (mut client, _, _, storage_ticket) = setup_push_metadata("create_grant").await?;
        storage_ticket.create_grant(&mut client).await?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn authorization_grants() -> Result<(), TombError> {
        let (mut client, omni, metadata, storage_ticket) =
            setup_push_metadata("authorization_grants").await?;
        let local = omni.get_local()?;
        let remote = omni.get_remote()?;

        // Create a grant
        storage_ticket.create_grant(&mut client).await?;
        // Assert 404 before any space has been allocated
        assert!(remote.get_grants_token(&mut client).await.is_err());
        // Upload content
        local
            .content
            .upload(storage_ticket.host, metadata.id, &mut client)
            .await?;
        // Successfully get a new bearer token which can access the new grants
        remote.get_grants_token(&mut client).await?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn get_locations() -> Result<(), TombError> {
        let (mut client, omni, metadata, storage_ticket) =
            setup_push_metadata("get_locations").await?;
        let local = omni.get_local()?;
        let wrapping_key = GlobalConfig::from_disk().await?.wrapping_key().await?;
        let mut fs = local.unlock_fs(&wrapping_key).await?;
        // Create a grant
        storage_ticket.clone().create_grant(&mut client).await?;
        local
            .content
            .upload(storage_ticket.host.clone(), metadata.id, &mut client)
            .await?;

        let mut blockstore_client = client.clone();
        blockstore_client
            .with_remote(&storage_ticket.host)
            .expect("Failed to create blockstore client");
        let banyan_api_blockstore = BanyanApiBlockStore::from(blockstore_client);

        let node = fs
            .get_node(&["cat.txt".to_string()], &local.metadata)
            .await?
            .unwrap();
        let file = node.as_file()?;
        let split_store = DoubleSplitStore::new(&banyan_api_blockstore, &local.metadata);
        let cids = file.get_cids(&fs.forest, &split_store).await?;
        let cids_request: LocationRequest = cids
            .clone()
            .into_iter()
            .map(|cid| cid.to_string())
            .collect();
        let locations = client
            .call(cids_request)
            .await
            .expect("Failed to get locations");
        let stored_blocks = locations
            .get(&storage_ticket.host)
            .expect("no blocks at storage host");
        for cid in cids {
            assert!(stored_blocks.contains(&cid.to_string()));
        }
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn get_bad_location() -> Result<(), TombError> {
        let mut client = authenticated_client().await;
        let cids: LocationRequest = vec![Cid::default().to_string()];
        let locations = client
            .call(cids.clone())
            .await
            .expect("Failed to get locations");
        let target_cids = locations.get("NA").expect("Failed to get cids");
        for cid in cids.clone() {
            assert!(target_cids.contains(&cid));
        }
        Ok(())
    }
}
