use crate::{
    cli::{
        commands::{prompt_for_bool, KeyCommand, MetadataCommand, RunnableCommand},
        specifiers::DriveSpecifier,
    },
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
use serde::{Deserialize, Serialize};
use std::{env::current_dir, path::PathBuf};

/// Subcommand for Drive Management
#[derive(Subcommand, Clone, Debug, Serialize, Deserialize)]
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
impl RunnableCommand<NativeError> for DrivesCommand {
    async fn run_internal(self) -> Result<String, NativeError> {
        match self {
            // List all Buckets tracked remotely and locally
            DrivesCommand::Ls => {
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
            DrivesCommand::Create { name, origin } => {
                let origin = origin.unwrap_or(current_dir()?);
                let omni = OmniBucket::create(&name, &origin).await?;
                let output = format!("{}\n{}", "<< NEW DRIVE CREATED >>".green(), omni);
                Ok(output)
            }
            DrivesCommand::Prepare {
                drive_specifier,
                follow_links,
            } => {
                prepare::pipeline(
                    OmniBucket::from_specifier(&drive_specifier).await,
                    follow_links,
                )
                .await
            }
            DrivesCommand::Restore { drive_specifier } => {
                restore::pipeline(OmniBucket::from_specifier(&drive_specifier).await).await
            }
            DrivesCommand::Sync(drive_specifier) => {
                OmniBucket::from_specifier(&drive_specifier)
                    .await
                    .sync_bucket()
                    .await
            }
            DrivesCommand::Delete(drive_specifier) => {
                let omni = OmniBucket::from_specifier(&drive_specifier).await;
                let local_deletion = prompt_for_bool("Do you want to delete this Bucket locally?");
                let remote_deletion =
                    prompt_for_bool("Do you want to delete this Bucket remotely?");
                omni.delete(local_deletion, remote_deletion).await
            }
            DrivesCommand::Info(drive_specifier) => {
                let omni = OmniBucket::from_specifier(&drive_specifier).await;
                Ok(format!("{omni}"))
            }
            DrivesCommand::Usage(drive_specifier) => {
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
            DrivesCommand::Metadata { subcommand } => subcommand.run_internal().await,
            DrivesCommand::Keys { subcommand } => subcommand.run_internal().await,
        }
    }
}
