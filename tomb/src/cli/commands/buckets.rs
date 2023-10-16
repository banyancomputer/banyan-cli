use crate::{
    pipelines::{bundle, error::TombError, extract},
    types::config::{bucket::OmniBucket, globalconfig::GlobalConfig},
};

use super::{super::specifiers::BucketSpecifier, KeyCommand, MetadataCommand, RunnableCommand};
use async_trait::async_trait;
use clap::Subcommand;
use colored::Colorize;
use std::{env::current_dir, path::PathBuf};
use tomb_common::{
    banyan_api::{
        client::Client,
        models::{bucket::Bucket, metadata::Metadata},
        requests::staging::upload::push::PushContent,
    },
    metadata::FsMetadata,
};

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
        output: PathBuf,
    },
    /// Push prepared Bucket data
    Push(BucketSpecifier),
    /// Download prepared Bucket data
    Pull(BucketSpecifier),
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
                Ok(format!(
                    "{}\n{}\n",
                    "<< NEW BUCKET CREATED >>".green(),
                    omni
                ))
            }
            BucketsCommand::Prepare {
                bucket_specifier,
                follow_links,
            } => bundle::pipeline(global, &bucket_specifier, follow_links).await,
            BucketsCommand::Restore {
                bucket_specifier,
                output,
            } => extract::pipeline(global, &bucket_specifier, &output).await,
            BucketsCommand::Push(bucket_specifier) => {
                // Obtain the bucket
                let _bucket = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                todo!()
            }
            BucketsCommand::Pull(_bucket_specifier) => {
                todo!()
            }
            BucketsCommand::Delete(bucket_specifier) => {
                let bucket = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                bucket.delete(global, client).await
            }
            BucketsCommand::Info(bucket_specifier) => {
                let bucket = OmniBucket::from_specifier(global, client, &bucket_specifier).await;
                Ok(format!("{bucket}"))
            }
            BucketsCommand::Usage(bucket_specifier) => {
                let bucket_id = global.get_bucket_id(&bucket_specifier)?;
                Bucket::read(client, bucket_id)
                    .await?
                    .usage(client)
                    .await
                    .map(|v| format!("id:\t{}\nusage:\t{}", bucket_id, v))
                    .map_err(TombError::client_error)
            }
            BucketsCommand::Metadata { subcommand } => {
                subcommand.run_internal(global, client).await
            }
            BucketsCommand::Keys { subcommand } => subcommand.run_internal(global, client).await,
        }
    }
}
