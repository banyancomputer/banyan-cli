use crate::cli2::commands::RunnableCommand;
use crate::{
    cli2::{prompt_for_bool, specifiers::DriveSpecifier},
    native::{
        configuration::globalconfig::GlobalConfig,
        operations::{prepare, restore},
        sync::OmniBucket,
        NativeError,
    },
};
use async_trait::async_trait;
use bytesize::ByteSize;
use clap::Subcommand;
use colored::Colorize;
use futures::executor::block_on;
use serde::{Deserialize, Serialize};
use std::{env::current_dir, path::PathBuf};

/// Subcommand for Drive Management
#[derive(Subcommand, Clone, Debug, Serialize, Deserialize)]
pub enum SyncCommand {
    /// List all managed synced drives
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
}

#[async_trait]
impl RunnableCommand<NativeError> for SyncCommand {
    async fn run_internal(self) -> Result<String, NativeError> {
        match self {
            // List all Buckets tracked remotely and locally
            SyncCommand::Ls => {
                let omnis = OmniBucket::ls().await?;
                if !omnis.is_empty() {
                    Ok(omnis
                        .iter()
                        .fold(String::new(), |acc, bucket| format!("{acc}\n{bucket}")))
                } else {
                    Ok("No known Drives locally or remotely.".to_string())
                }
            }
            // Create a new Bucket. This attempts to create the Bucket both locally and remotely, but settles for a simple local creation if remote permissions fail
            SyncCommand::Create { name, origin } => {
                let origin = origin.unwrap_or(current_dir()?);
                let omni = OmniBucket::create(&name, &origin).await?;
                let output = format!("{}\n{}", "<< NEW DRIVE CREATED >>".green(), omni);
                Ok(output)
            }
            SyncCommand::Prepare {
                drive_specifier,
                follow_links,
            } => block_on(prepare::pipeline(
                OmniBucket::from_specifier(&drive_specifier).await,
                follow_links,
            )),
            SyncCommand::Restore { drive_specifier } => block_on(restore::pipeline(
                OmniBucket::from_specifier(&drive_specifier).await,
            )),
            SyncCommand::Sync(drive_specifier) => block_on(
                OmniBucket::from_specifier(&drive_specifier)
                    .await
                    .sync_bucket(),
            ),
            SyncCommand::Delete(drive_specifier) => {
                let omni = OmniBucket::from_specifier(&drive_specifier).await;
                let local_deletion = prompt_for_bool("Do you want to delete this Bucket locally?");
                let remote_deletion =
                    prompt_for_bool("Do you want to delete this Bucket remotely?");
                omni.delete(local_deletion, remote_deletion).await
            }
            SyncCommand::Info(drive_specifier) => {
                let omni = OmniBucket::from_specifier(&drive_specifier).await;
                Ok(format!("{omni}"))
            }
            SyncCommand::Usage(drive_specifier) => {
                let mut client = GlobalConfig::from_disk().await?.get_client().await?;
                let remote = OmniBucket::from_specifier(&drive_specifier)
                    .await
                    .get_remote()?;
                remote
                    .usage(&mut client)
                    .await
                    .map(|v| {
                        format!(
                            "{}drive_id:\t\t{}\nusage:\t\t{}",
                            "| USAGE INFO |".blue(),
                            remote.id,
                            ByteSize(v)
                        )
                    })
                    .map_err(NativeError::api)
            }
        }
    }
}
