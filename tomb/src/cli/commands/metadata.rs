use crate::{
    pipelines::error::TombError,
    types::config::{bucket::OmniBucket, globalconfig::GlobalConfig},
    utils::wnfsio::compute_directory_size,
};

use super::{super::specifiers::*, RunnableCommand};
use async_trait::async_trait;
use clap::Subcommand;
use futures_util::StreamExt;
use tomb_common::{
    banyan_api::{client::Client, error::ClientError, models::metadata::Metadata},
    blockstore::RootedBlockStore,
    metadata::FsMetadata,
};

/// Subcommand for Bucket Metadata
#[derive(Subcommand, Clone, Debug)]
pub enum MetadataCommand {
    /// Read an individual Metadata Id
    Read(MetadataSpecifier),
    /// Read the currently active Metadata
    ReadCurrent(BucketSpecifier),
    /// List all Metadatas associated with Bucket
    List(BucketSpecifier),
    /// Upload Metadata
    Push(BucketSpecifier),
    /// Download Metadata
    Pull(MetadataSpecifier),
    /// Grab Snapshot
    Snapshot(MetadataSpecifier),
}

#[async_trait(?Send)]
impl RunnableCommand<TombError> for MetadataCommand {
    async fn run_internal(
        self,
        global: &mut GlobalConfig,
        client: &mut Client,
    ) -> Result<String, TombError> {
        match self {
            // Read an existing metadata
            MetadataCommand::Read(metadata_specifier) => {
                // Get Bucket config
                let omni = OmniBucket::from_specifier(global, client, &metadata_specifier.bucket_specifier).await;
                // If we can get the metadata
                if let Some(remote_id) = omni.get_id() {
                    Metadata::read(remote_id, metadata_specifier.metadata_id, client)
                        .await
                        .map(|metadata| format!("{:?}", metadata))
                        .map_err(TombError::client_error)
                } else {
                    Err(TombError::custom_error("Config has no remote id!"))
                }
            }
            // Push metadata
            MetadataCommand::Push(bucket_specifier) => {
                // Get info
                let wrapping_key: tomb_crypt::prelude::EcEncryptionKey =
                    global.wrapping_key().await?;
                let omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                if let Some(local) = omni.local {
                    let fs = FsMetadata::unlock(&wrapping_key, &local.metadata).await?;
                    let valid_keys = fs.share_manager.public_fingerprints();
                    let expected_data_size = compute_directory_size(&local.metadata.path)? as u64;
                    let bucket_id = local.remote_id.expect("no remote id");
                    let root_cid = local.metadata.get_root().expect("no root cid").to_string();
                    let metadata_cid = local
                        .metadata
                        .get_root()
                        .expect("no metadata cid")
                        .to_string();
                    let metadata_stream = tokio::fs::File::open(&local.metadata.path).await?;
                    // Push the Metadata
                    Metadata::push(
                        bucket_id,
                        root_cid,
                        metadata_cid,
                        expected_data_size,
                        valid_keys,
                        metadata_stream,
                        client,
                    )
                    .await
                    .map(|(metadata, storage_ticket)| {
                        let mut info = format!("\t{}", metadata);
                        if let Some(storage_ticket) = storage_ticket {
                            info.push_str(&format!("\n\n\t{}", storage_ticket))
                        }
                        info
                    })
                    .map_err(TombError::client_error)
                } else {
                    Ok("no bucket".to_string())
                }
            }
            // Read the current Metadata
            MetadataCommand::ReadCurrent(bucket_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                let bucket_id = omni.get_id().expect("no remote id");
                Metadata::read_current(bucket_id, client)
                    .await
                    .map(|metadata| format!("{:?}", metadata))
                    .map_err(TombError::client_error)
            }
            // List all Metadata for a Bucket
            MetadataCommand::List(bucket_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                let bucket_id = omni.get_id().expect("no remote id");
                Metadata::read_all(bucket_id, client)
                    .await
                    .map(|metadatas| {
                        metadatas
                            .iter()
                            .fold("<< METADATAS >>".to_string(), |acc, metadata| {
                                format!("{}{}", acc, metadata)
                            })
                    })
                    .map_err(TombError::client_error)
            }
            // Pull a Metadata and replace the local copy
            MetadataCommand::Pull(metadata_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &metadata_specifier.bucket_specifier).await;
                let bucket_id = omni.get_id().expect("no remote id");
                let metadata =
                    Metadata::read(bucket_id, metadata_specifier.metadata_id, client).await?;
                let mut byte_stream = metadata.pull(client).await?;
                let mut file = tokio::fs::File::create(&omni.local.unwrap().metadata.path).await?;

                while let Some(chunk) = byte_stream.next().await {
                    tokio::io::copy(
                        &mut chunk.map_err(ClientError::http_error)?.as_ref(),
                        &mut file,
                    )
                    .await?;
                }

                Ok("successfully downloaded metadata".to_string())
            }
            // Take a Cold Snapshot of the remote metadata
            MetadataCommand::Snapshot(metadata_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &metadata_specifier.bucket_specifier).await;
                let bucket_id = omni.get_id().expect("no remote id");
                let metadata =
                    Metadata::read(bucket_id, metadata_specifier.metadata_id, client).await?;

                metadata
                    .snapshot(client)
                    .await
                    .map(|snapshot| format!("{:?}", snapshot))
                    .map_err(TombError::client_error)
            }
        }
    }
}
