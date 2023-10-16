use crate::{
    pipelines::{bundle, error::TombError, extract},
    types::config::{bucket::OmniBucket, globalconfig::GlobalConfig},
};

use super::{super::specifiers::BucketSpecifier, KeyCommand, MetadataCommand, RunnableCommand};
use async_trait::async_trait;
use clap::Subcommand;
use std::{env::current_dir, path::PathBuf};
use tomb_common::banyan_api::{
    client::Client,
    models::bucket::{Bucket, BucketType, StorageClass},
};
use tomb_crypt::prelude::{PrivateKey, PublicKey};

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
                let private_key = global.wrapping_key().await?;
                let public_key = private_key.public_key()?;
                let pem = String::from_utf8(public_key.export().await?)
                    .map_err(|_| TombError::custom_error("unable to represent pem from utf8"))?;
                let origin = origin.unwrap_or(current_dir()?);
                // If this bucket already exists both locally and remotely
                if let Some(bucket) = global.get_bucket_by_origin(&origin) &&
                    let Some(remote_id) = bucket.remote_id &&
                    Bucket::read(client, remote_id).await.is_ok() {
                    // If we are able to read the bucket
                    return Err(TombError::custom_error("Bucket already exists at this origin and is persisted remotely"));
                }

                // Initialize in the configs
                let mut config = global.get_or_create_bucket(&origin).await?;

                // Update the config globally
                global
                    .update_config(&config)
                    .expect("unable to update config to include local path");

                // Initialize on the remote endpoint
                let online_result = Bucket::create(
                    name.to_string(),
                    pem,
                    BucketType::Interactive,
                    StorageClass::Hot,
                    client,
                )
                .await
                .map(|(bucket, key)| {
                    // Update the bucket config id
                    config.remote_id = Some(bucket.id);
                    // Update the config globally
                    global
                        .update_config(&config)
                        .expect("unable to update config to include remote id");
                    // Return
                    format!("<< NEW BUCKET CREATED >>\n{bucket}\n{config}\n{key}")
                })
                .map_err(TombError::client_error);

                if let Ok(string) = online_result {
                    Ok(string)
                } else {
                    Ok(format!("<< NEW BUCKET CREATED (LOCAL ONLY) >>\n{config}"))
                }
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
