use crate::{
    pipelines::{bundle, error::TombError, extract},
    types::config::globalconfig::GlobalConfig,
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
    /// Initialize a new Bucket locally
    Create {
        /// Bucket Name
        #[arg(short, long)]
        name: String,
        /// Bucket Root
        #[arg(short, long)]
        origin: Option<PathBuf>,
    },
    /// Encrypt / Bundle a Bucket
    Bundle {
        /// Bucket in question
        #[clap(flatten)]
        bucket_specifier: BucketSpecifier,

        /// Follow symbolic links
        #[arg(short, long)]
        follow_links: bool,
    },
    /// Decrypt / Extract a Bucket
    Extract {
        /// Bucket in question
        #[clap(flatten)]
        bucket_specifier: BucketSpecifier,

        /// Output Directory
        #[arg(short, long)]
        output: PathBuf,
    },
    /// List all Buckets
    List,
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
            // Bundle a local directory
            BucketsCommand::Bundle {
                bucket_specifier,
                follow_links,
            } => bundle::pipeline(global, &bucket_specifier, follow_links).await,
            // Extract a local directory
            BucketsCommand::Extract {
                bucket_specifier,
                output,
            } => extract::pipeline(global, &bucket_specifier, &output).await,
            // List all Buckets tracked remotely and locally
            BucketsCommand::List => {
                let local = global
                    .buckets
                    .iter()
                    .fold("<< LOCAL BUCKETS >>".to_string(), |acc, bucket| {
                        format!("{acc}{bucket}")
                    });

                let remote = Bucket::read_all(client)
                    .await
                    .map(|buckets| {
                        buckets
                            .iter()
                            .fold("<< REMOTE BUCKETS >>".to_string(), |acc, bucket| {
                                format!("{acc}{bucket}")
                            })
                    })
                    .map_err(TombError::client_error)?;
                Ok(format!("{}\n\n{}", remote, local))
            }
            // Delete a Bucket
            BucketsCommand::Delete(bucket_specifier) => {
                // If we're online and there is a known bucket id with this specifier
                let remote_deletion = if let Ok(bucket_id) = global.get_bucket_id(&bucket_specifier)
                {
                    Bucket::delete_by_id(client, bucket_id).await.is_ok()
                } else {
                    false
                };

                // Remove the bucket locally if it is known
                let local_deletion = if global.get_bucket_by_specifier(&bucket_specifier).is_ok() {
                    // Remove the Bucket locally
                    global.remove_bucket_by_specifier(&bucket_specifier).is_ok()
                } else {
                    false
                };

                Ok(format!(
                    "<< BUCKET DELETION >>\nlocal:\t{local_deletion}\nremote:\t{remote_deletion}"
                ))
            }
            // Info about a Bucket
            BucketsCommand::Info(bucket_specifier) => {
                // Local info
                let local = if let Ok(bucket) = global.get_bucket_by_specifier(&bucket_specifier) {
                    format!("{}", bucket)
                } else {
                    "no known local bucket".to_string()
                };

                // If there is known remote counterpart to the Bucket
                let remote = if let Ok(id) = global.get_bucket_id(&bucket_specifier) {
                    match Bucket::read(client, id).await {
                        Ok(bucket) => {
                            format!("{bucket}")
                        }
                        Err(err) => format!("error: {}", err),
                    }
                } else {
                    "no known remote bucket".to_string()
                };

                Ok(format!(
                    "<< BUCKET INFO >>\nlocal:\t{}\nremote:\t{}",
                    local, remote
                ))
            }
            // Bucket usage
            BucketsCommand::Usage(bucket_specifier) => {
                let bucket_id = global.get_bucket_id(&bucket_specifier)?;
                Bucket::read(client, bucket_id)
                    .await?
                    .usage(client)
                    .await
                    .map(|v| format!("id:\t{}\nusage:\t{}", bucket_id, v))
                    .map_err(TombError::client_error)
            }
            // Bucket Metadata
            BucketsCommand::Metadata { subcommand } => {
                subcommand.run_internal(global, client).await
            }
            // Bucket Key Management
            BucketsCommand::Keys { subcommand } => subcommand.run_internal(global, client).await,
        }
    }
}
