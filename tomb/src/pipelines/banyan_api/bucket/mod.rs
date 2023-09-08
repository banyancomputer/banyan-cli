use crate::{
    cli::command::*,
    pipelines::{bundle, error::TombError, extract},
    types::config::globalconfig::GlobalConfig,
};
use anyhow::{anyhow, Result};
use std::env::current_dir;
use tomb_common::banyan_api::{
    client::Client,
    models::bucket::{Bucket, BucketType, StorageClass},
};
use tomb_crypt::prelude::{PrivateKey, PublicKey};

pub(crate) mod keys;
pub(crate) mod metadata;

/// Handle Bucket management both locally and remotely based on CLI input
pub async fn pipeline(command: BucketsSubCommand) -> Result<String> {
    // Grab global config
    let mut global = GlobalConfig::from_disk().await?;
    // Obtain the Client
    let client: &mut Option<Client> = &mut global.get_client().await.ok();

    // if let Some(client) = client {

    // }

    // Process the command
    let result: Result<String, TombError> = match command {
        // Create a new Bucket. This attempts to create the Bucket both locally and remotely, but settles for a simple local creation if remote permissions fail
        BucketsSubCommand::Create { name, origin } => {
            let private_key = global.wrapping_key().await?;
            let public_key = private_key.public_key()?;
            let pem = String::from_utf8(public_key.export().await?)?;

            let origin = &origin.unwrap_or(current_dir()?);

            // If this bucket already exists both locally and remotely
            if let Some(bucket) = global.get_bucket_by_origin(origin) && let Some(remote_id) = bucket.remote_id && let Some(client) = client && Bucket::read(client, remote_id).await.is_ok() {
                // If we are able to read the bucket
                return Err(anyhow!("Bucket already exists at this origin and is persisted remotely"));
            }

            // Initialize in the configs
            let mut config = global.get_or_create_bucket(origin).await?;

            // Update the config globally
            global
                .update_config(&config)
                .expect("unable to update config to include local path");

            // If we're online
            if let Some(client) = client {
                // Initialize on the remote endpoint
                Bucket::create(
                    name,
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
                .map_err(TombError::client_error)
            } else {
                Ok(format!("<< NEW BUCKET CREATED (LOCAL ONLY) >>\n{config}"))
            }
        }
        // Bundle a local directory
        BucketsSubCommand::Bundle {
            bucket_specifier,
            follow_links,
        } => bundle::pipeline(&mut global, &bucket_specifier, follow_links).await,
        // Extract a local directory
        BucketsSubCommand::Extract {
            bucket_specifier,
            output,
        } => extract::pipeline(&global, &bucket_specifier, &output).await,
        // List all Buckets tracked remotely and locally
        BucketsSubCommand::List => {
            let local = global
                .buckets
                .iter()
                .fold("<< LOCAL BUCKETS >>".to_string(), |acc, bucket| {
                    format!("{acc}{bucket}")
                });

            if let Some(client) = client {
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
            } else {
                Ok(format!("{}\n", local))
            }
        }
        // Delete a Bucket
        BucketsSubCommand::Delete(bucket_specifier) => {
            // If we're online and there is a known bucket id with this specifier
            let remote_deletion = if let Some(client) = client && let Ok(bucket_id) = global.get_bucket_id(&bucket_specifier) {
                Bucket::delete_by_id(client, bucket_id)
                    .await.is_ok()
            } else { false };

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
        BucketsSubCommand::Info(bucket_specifier) => {
            // Local info
            let local = if let Ok(bucket) = global.get_bucket_by_specifier(&bucket_specifier) {
                format!("{}", bucket)
            } else {
                "no known local bucket".to_string()
            };

            // If there is known remote counterpart to the Bucket
            let remote = if let Some(client) = client && let Ok(id) = global.get_bucket_id(&bucket_specifier) {
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
        BucketsSubCommand::Usage(bucket_specifier) => {
            let bucket_id = global.get_bucket_id(&bucket_specifier)?;
            if let Some(client) = client {
                Bucket::read(client, bucket_id)
                    .await?
                    .usage(client)
                    .await
                    .map(|v| format!("id:\t{}\nusage:\t{}", bucket_id, v))
                    .map_err(TombError::client_error)
            } else {
                Err(anyhow!("cannot check usage offline").into())
            }
        }
        // Bucket Metadata
        BucketsSubCommand::Metadata { subcommand } => {
            if let Some(client) = client {
                metadata::pipeline(&global, client, subcommand).await
            } else {
                Err(anyhow!("cannot perform metadata operations offline").into())
            }
        }
        // Bucket Key Management
        BucketsSubCommand::Keys { subcommand } => {
            if let Some(client) = client {
                keys::pipeline(&global, client, subcommand).await
            } else {
                Err(anyhow!("cannot perform key management operations offline").into())
            }
        }
    };

    // If there is a client to update and save
    if let Some(client) = client {
        // Save the Client
        global.save_client(client.clone()).await?;
    }
    global.to_disk()?;

    // Return
    result.map_err(anyhow::Error::new)
}
