use crate::banyan_cli::{
    pipelines::error::TombError,
    types::config::{bucket::OmniBucket, globalconfig::GlobalConfig},
};
use crate::banyan_common::banyan_api::{client::Client, models::metadata::Metadata};

use super::{super::specifiers::*, RunnableCommand};
use async_trait::async_trait;
use clap::Subcommand;

/// Subcommand for Bucket Metadata
#[derive(Subcommand, Clone, Debug)]
pub enum MetadataCommand {
    /// List all Metadatas associated with Bucket
    Ls(DriveSpecifier),
    /// Read an individual Metadata Id
    Read(MetadataSpecifier),
    /// Read the currently active Metadata
    ReadCurrent(DriveSpecifier),
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
            // List all Metadata for a Bucket
            MetadataCommand::Ls(drive_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &drive_specifier).await;
                let bucket_id = omni.get_id().expect("no remote id");
                Metadata::read_all(bucket_id, client)
                    .await
                    .map(|metadatas| {
                        metadatas.iter().fold(String::from("\n"), |acc, metadata| {
                            format!("{}\n\n{}", acc, metadata)
                        })
                    })
                    .map_err(TombError::client_error)
            }
            // Read an existing metadata
            MetadataCommand::Read(metadata_specifier) => {
                // Get Bucket config
                let omni =
                    OmniBucket::from_specifier(global, client, &metadata_specifier.drive_specifier)
                        .await;
                // If we can get the metadata
                let remote_id = omni.get_id()?;
                Metadata::read(remote_id, metadata_specifier.metadata_id, client)
                    .await
                    .map(|metadata| format!("{:?}", metadata))
                    .map_err(TombError::client_error)
            }
            // Read the current Metadata
            MetadataCommand::ReadCurrent(drive_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &drive_specifier).await;
                let bucket_id = omni.get_id().expect("no remote id");
                Metadata::read_current(bucket_id, client)
                    .await
                    .map(|metadata| format!("{:?}", metadata))
                    .map_err(TombError::client_error)
            }
            // Take a Cold Snapshot of the remote metadata
            MetadataCommand::Snapshot(metadata_specifier) => {
                let omni =
                    OmniBucket::from_specifier(global, client, &metadata_specifier.drive_specifier)
                        .await;
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
