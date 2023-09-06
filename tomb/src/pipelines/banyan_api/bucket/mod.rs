use crate::{
    cli::command::*,
    pipelines::{bundle, error::TombError, extract},
    types::config::globalconfig::GlobalConfig,
};
use anyhow::{anyhow, Result};
use std::env::current_dir;
use tomb_common::banyan_api::models::bucket::{Bucket, BucketType, StorageClass};
use tomb_crypt::prelude::{EcEncryptionKey, PrivateKey, PublicKey};

pub(crate) mod keys;
pub(crate) mod metadata;
pub(crate) mod snapshots;

/// Handle Bucket management both locally and remotely based on CLI input
pub async fn pipeline(command: BucketsSubCommand) -> Result<String> {
    // Grab global config
    let mut global = GlobalConfig::from_disk().await?;
    // Obtain the Client
    let mut client = global.get_client().await?;

    // Process the command
    let result: Result<String, TombError> = match command {
        // Create a new Bucket. This attempts to create the Bucket both locally and remotely, but settles for a simple local creation if remote permissions fail
        BucketsSubCommand::Create { name, origin } => {
            let private_key = EcEncryptionKey::generate().await?;
            let public_key = private_key.public_key()?;
            let pem = String::from_utf8(public_key.export().await?)?;

            let origin = &origin.unwrap_or(current_dir()?);

            // If we've already done this
            if let Some(bucket) = global.get_bucket_by_origin(origin) && let Some(remote_id) = bucket.remote_id && Bucket::read(&mut client, remote_id).await.is_ok() {
                // If we are able to read the bucket
                return Err(anyhow!("Bucket already exists at this origin and is persisted remotely"));
            }

            // Initialize in the configs
            let mut config = global.get_or_create_bucket(origin).await?;

            // Update the config globally
            global
                .update_config(&config)
                .expect("unable to update config to include local path");

            // Initialize on the remote endpoint
            Bucket::create(
                name,
                pem,
                BucketType::Interactive,
                StorageClass::Hot,
                &mut client,
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
                format!("<<NEW BUCKET CREATED >>\n{}\n{}\n{}", bucket, config, key)
            })
            .map_err(TombError::client_error)
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
            let remote = Bucket::read_all(&mut client)
                .await
                .map(|buckets| {
                    buckets
                        .iter()
                        .fold("<< REMOTE BUCKETS >>".to_string(), |acc, bucket| {
                            format!("{}{}", acc, bucket)
                        })
                })
                .map_err(TombError::client_error)?;

            let local = global
                .buckets
                .iter()
                .fold("<< LOCAL BUCKETS >>".to_string(), |acc, bucket| {
                    format!("{}{}", acc, bucket)
                });

            Ok(format!("{}\n\n{}", remote, local))
        }
        // Delete a Bucket
        BucketsSubCommand::Delete(bucket_specifier) => {
            // Rmove the Bucket locally
            global.remove_bucket_by_specifier(&bucket_specifier)?;
            // Remove the bucket remotely
            let bucket_id = global.get_bucket_id(&bucket_specifier)?;
            Bucket::delete_by_id(&mut client, bucket_id)
                .await
                .map(|_| "bucket deleted".to_string())
                .map_err(TombError::client_error)
        }
        // Info about a Bucket
        BucketsSubCommand::Info(bucket_specifier) => {
            // If there is known remote counterpart to the Bucket
            let remote = if let Ok(id) = global.get_bucket_id(&bucket_specifier) {
                match Bucket::read(&mut client, id).await {
                    Ok(bucket) => {
                        format!("{}", bucket)
                    }
                    Err(err) => format!("error: {}", err),
                }
            } else {
                format!("no known remote correlate")
            };

            let local = if let Ok(bucket) = global.get_bucket_by_specifier(&bucket_specifier) {
                format!("{}", bucket)
            } else {
                "no known local bucket".to_string()
            };

            Ok(format!("{}{}", local, remote))
        }
        // Bucket usage
        BucketsSubCommand::Usage(bucket_specifier) => {
            let bucket_id = global.get_bucket_id(&bucket_specifier)?;
            Bucket::read(&mut client, bucket_id)
                .await?
                .usage(&mut client)
                .await
                .map(|v| format!("id:\t{}\nusage:\t{}", bucket_id, v))
                .map_err(TombError::client_error)
        }
        // Bucket Metadata
        BucketsSubCommand::Metadata { subcommand } => {
            metadata::pipeline(&global, &mut client, subcommand).await
        }
        // Bucket Key Management
        BucketsSubCommand::Keys { subcommand } => {
            keys::pipeline(&global, &mut client, subcommand).await
        }
    };

    // Save the Client
    global.save_client(client).await?;
    global.to_disk()?;

    // Return
    result.map_err(anyhow::Error::new)
}
