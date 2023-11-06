use super::{super::specifiers::DriveSpecifier, KeyCommand, MetadataCommand, RunnableCommand};
use crate::banyan_cli::{
    pipelines::{error::TombError, prepare, restore},
    types::config::{
        bucket::{sync_bucket, OmniBucket},
        globalconfig::GlobalConfig,
    },
};
use crate::banyan_common::{banyan_api::client::Client, metadata::FsMetadata};
use async_trait::async_trait;
use bytesize::ByteSize;
use clap::Subcommand;
use colored::Colorize;
use std::{env::current_dir, path::PathBuf};

/// Subcommand for Drive Management
#[derive(Subcommand, Clone, Debug)]
pub enum DrivesCommand {
    /// List all Drives
    Ls,
    /// Initialize a new Drive
    Create {
        /// Drive Name
        #[arg(short, long)]
        name: String,
        /// Drive Root
        #[arg(short, long)]
        origin: Option<PathBuf>,
    },
    /// Prepare a Drive for Pushing by encrypting new data
    Prepare {
        /// Drive in question
        #[clap(flatten)]
        drive_specifier: DriveSpecifier,

        /// Follow symbolic links
        #[arg(short, long)]
        follow_links: bool,
    },
    /// Reconstruct a Drive filesystem locally
    Restore {
        /// Drive in question
        #[clap(flatten)]
        drive_specifier: DriveSpecifier,
    },
    /// Sync Drive data to or from remote
    Sync(DriveSpecifier),
    /// Delete a Drive
    Delete(DriveSpecifier),
    /// Drive info
    Info(DriveSpecifier),
    /// Drive data usage
    Usage(DriveSpecifier),
    /// Get information on Drive Metadata
    Metadata {
        /// Subcommand
        #[clap(subcommand)]
        subcommand: MetadataCommand,
    },
    /// Drive Key management
    Keys {
        /// Subcommand
        #[clap(subcommand)]
        subcommand: KeyCommand,
    },
}

#[async_trait(?Send)]
impl RunnableCommand<TombError> for DrivesCommand {
    async fn run_internal(
        self,
        global: &mut GlobalConfig,
        client: &mut Client,
    ) -> Result<String, TombError> {
        match self {
            // List all Buckets tracked remotely and locally
            DrivesCommand::Ls => {
                let omnis = OmniBucket::ls(global, client).await;
                if !omnis.is_empty() {
                    Ok(omnis
                        .iter()
                        .fold(String::new(), |acc, bucket| format!("{acc}\n{bucket}")))
                } else {
                    Ok("No known Drives locally or remotely.".to_string())
                }
            }
            // Create a new Bucket. This attempts to create the Bucket both locally and remotely, but settles for a simple local creation if remote permissions fail
            DrivesCommand::Create { name, origin } => {
                let origin = origin.unwrap_or(current_dir()?);
                let omni = OmniBucket::create(global, client, &name, &origin).await?;
                let output = format!("{}\n{}", "<< NEW DRIVE CREATED >>".green(), omni);
                Ok(output)
            }
            DrivesCommand::Prepare {
                drive_specifier,
                follow_links,
            } => {
                let mut omni = OmniBucket::from_specifier(global, client, &drive_specifier).await;
                let fs =
                    FsMetadata::unlock(&global.wrapping_key().await?, &omni.get_local()?.metadata)
                        .await?;
                let result = prepare::pipeline(fs, &mut omni, client, follow_links).await;
                global.update_config(&omni.get_local()?)?;
                result
            }
            DrivesCommand::Restore { drive_specifier } => {
                let mut omni = OmniBucket::from_specifier(global, client, &drive_specifier).await;
                let fs =
                    FsMetadata::unlock(&global.wrapping_key().await?, &omni.get_local()?.metadata)
                        .await?;
                let result = restore::pipeline(fs, &mut omni, client).await;
                global.update_config(&omni.get_local()?)?;
                result
            }
            DrivesCommand::Sync(drive_specifier) => {
                let mut omni = OmniBucket::from_specifier(global, client, &drive_specifier).await;
                let result = sync_bucket(&mut omni, client, global).await;
                if let Ok(local) = omni.get_local() {
                    global.update_config(&local)?;
                }
                result
            }
            DrivesCommand::Delete(drive_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &drive_specifier).await;
                omni.delete(global, client).await
            }
            DrivesCommand::Info(drive_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &drive_specifier).await;
                Ok(format!("{omni}"))
            }
            DrivesCommand::Usage(drive_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &drive_specifier).await;
                let remote = omni.get_remote()?;
                remote
                    .usage(client)
                    .await
                    .map(|v| {
                        format!(
                            "{}drive_id:\t\t{}\nusage:\t\t{}",
                            "| USAGE INFO |".blue(),
                            remote.id,
                            ByteSize(v)
                        )
                    })
                    .map_err(TombError::client_error)
            }
            DrivesCommand::Metadata { subcommand } => subcommand.run_internal(global, client).await,
            DrivesCommand::Keys { subcommand } => subcommand.run_internal(global, client).await,
        }
    }
}
