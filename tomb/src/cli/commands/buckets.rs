use crate::{
    pipelines::{error::TombError, prepare, reconstruct},
    types::config::{
        bucket::{sync_bucket, OmniBucket},
        globalconfig::GlobalConfig,
    },
};

use super::{
    super::specifiers::BucketSpecifier, prompt_for_bool, KeyCommand, MetadataCommand,
    RunnableCommand,
};
use async_trait::async_trait;
use clap::Subcommand;
use colored::Colorize;
use std::{env::current_dir, fs::remove_dir_all, path::PathBuf};
use tomb_common::banyan_api::client::Client;

/// Subcommand for Bucket Management
#[derive(Subcommand, Clone, Debug)]
pub enum BucketsCommand {
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

        /// Output Directory
        #[arg(short, long)]
        restore_path: Option<PathBuf>,
    },
    /// Sync Bucket data
    Sync(BucketSpecifier),
    /// Delete Bucket
    Delete(BucketSpecifier),
    /// Bucket info
    Info(BucketSpecifier),
    /// Bucket usage
    Usage(BucketSpecifier),
    /// Metadata uploads and downloads
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
impl RunnableCommand<TombError> for BucketsCommand {
    async fn run_internal(
        self,
        global: &mut GlobalConfig,
        client: &mut Client,
    ) -> Result<String, TombError> {
        match self {
            // List all Buckets tracked remotely and locally
            BucketsCommand::Ls => {
                let omnis = OmniBucket::ls(global, client).await;
                let str = omnis
                    .iter()
                    .fold(String::new(), |acc, bucket| format!("{acc}\n{bucket}"));
                Ok(str)
            }
            // Create a new Bucket. This attempts to create the Bucket both locally and remotely, but settles for a simple local creation if remote permissions fail
            BucketsCommand::Create { name, origin } => {
                let origin = origin.unwrap_or(current_dir()?);
                let omni = OmniBucket::create(global, client, &name, &origin).await?;
                let output = format!("{}\n{}\n", "<< NEW BUCKET CREATED >>".green(), omni);
                Ok(output)
            }
            BucketsCommand::Prepare {
                bucket_specifier,
                follow_links,
            } => {
                let omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                let local = omni.get_local()?;
                prepare::pipeline(global, local, follow_links).await
            }
            BucketsCommand::Restore {
                bucket_specifier,
                restore_path,
            } => {
                let omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                let local = omni.get_local()?;
                if let Some(restore_path) = restore_path {
                    reconstruct::pipeline(global, &local, &local.content, &restore_path).await
                } else if prompt_for_bool("delete data currently in unprepared Bucket origin?") {
                    remove_dir_all(&local.origin)?;
                    reconstruct::pipeline(global, &local, &local.content, &local.origin).await
                } else {
                    Ok("did nothing".into())
                }
            }
            BucketsCommand::Sync(bucket_specifier) => {
                let mut omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                let result = sync_bucket(&mut omni, client, global).await;
                if let Ok(local) = omni.get_local() {
                    global.update_config(&local)?;
                }
                result
            }
            BucketsCommand::Delete(bucket_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                omni.delete(global, client).await
            }
            BucketsCommand::Info(bucket_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                Ok(format!("{omni}"))
            }
            BucketsCommand::Usage(bucket_specifier) => {
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
                            v
                        )
                    })
                    .map_err(TombError::client_error)
            }
            BucketsCommand::Metadata { subcommand } => {
                subcommand.run_internal(global, client).await
            }
            BucketsCommand::Keys { subcommand } => subcommand.run_internal(global, client).await,
        }
    }
}
