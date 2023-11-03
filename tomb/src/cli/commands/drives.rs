use super::{super::specifiers::BucketSpecifier, KeyCommand, MetadataCommand, RunnableCommand};
use crate::{
    pipelines::{error::TombError, prepare, restore},
    types::config::{
        bucket::{sync_bucket, OmniBucket},
        globalconfig::GlobalConfig,
    },
};
use async_trait::async_trait;
use bytesize::ByteSize;
use clap::Subcommand;
use colored::Colorize;
use std::{env::current_dir, path::PathBuf};
use tomb_common::{banyan_api::client::Client, metadata::FsMetadata};

/// Subcommand for Bucket Management
#[derive(Subcommand, Clone, Debug)]
pub enum DrivesCommand {
    /// List all Buckets
    Ls,
    /// Initialize a new Bucket locally
    Create {
        /// Bucket Name
        #[arg(short, long)]
        name: String,
        /// Bucket Root
        #[arg(short, long)]
        origin: Option<PathBuf>,
    },
    /// Prepare a Bucket for Pushing by encrypting new data
    Prepare {
        /// Bucket in question
        #[clap(flatten)]
        bucket_specifier: BucketSpecifier,

        /// Follow symbolic links
        #[arg(short, long)]
        follow_links: bool,
    },
    /// Reconstruct a filesystem using an encrypted Bucket
    Restore {
        /// Bucket in question
        #[clap(flatten)]
        bucket_specifier: BucketSpecifier,
    },
    /// Sync Bucket data to or from remote
    Sync(BucketSpecifier),
    /// Delete Bucket
    Delete(BucketSpecifier),
    /// Bucket info
    Info(BucketSpecifier),
    /// Bucket data usage
    Usage(BucketSpecifier),
    /// Get information on Bucket Metadata
    Metadata {
        /// Subcommand
        #[clap(subcommand)]
        subcommand: MetadataCommand,
    },
    /// Bucket Key management
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
                let output = format!("{}\n{}", "<< NEW BUCKET CREATED >>".green(), omni);
                Ok(output)
            }
            DrivesCommand::Prepare {
                bucket_specifier,
                follow_links,
            } => {
                let mut omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                let fs =
                    FsMetadata::unlock(&global.wrapping_key().await?, &omni.get_local()?.metadata)
                        .await?;
                let result = prepare::pipeline(fs, &mut omni, client, follow_links).await;
                global.update_config(&omni.get_local()?)?;
                result
            }
            DrivesCommand::Restore { bucket_specifier } => {
                let mut omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                let fs =
                    FsMetadata::unlock(&global.wrapping_key().await?, &omni.get_local()?.metadata)
                        .await?;
                let result = restore::pipeline(fs, &mut omni, client).await;
                global.update_config(&omni.get_local()?)?;
                result
            }
            DrivesCommand::Sync(bucket_specifier) => {
                let mut omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                let result = sync_bucket(&mut omni, client, global).await;
                if let Ok(local) = omni.get_local() {
                    global.update_config(&local)?;
                }
                result
            }
            DrivesCommand::Delete(bucket_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                omni.delete(global, client).await
            }
            DrivesCommand::Info(bucket_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                Ok(format!("{omni}"))
            }
            DrivesCommand::Usage(bucket_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                let remote = omni.get_remote()?;
                remote
                    .usage(client)
                    .await
                    .map(|v| {
                        format!(
                            "{}bucket_id:\t\t{}\nusage:\t\t{}",
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
