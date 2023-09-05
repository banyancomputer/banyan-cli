use std::env::current_dir;
use crate::{
    cli::command::*,
    pipelines::{bundle, error::TombError, extract},
    types::config::globalconfig::GlobalConfig,
    utils::wnfsio::compute_directory_size,
};
use anyhow::{anyhow, Result};
use reqwest::Body;
use tokio::io::AsyncWriteExt;
use tomb_common::{
    banyan_api::models::{
        bucket::{Bucket, BucketType, StorageClass},
        bucket_key::BucketKey,
        metadata::Metadata,
        storage_ticket,
    },
    blockstore::RootedBlockStore,
    utils::io::get_read, metadata::FsMetadata,
};
use futures_util::stream::StreamExt;
use tomb_crypt::prelude::{EcEncryptionKey, PrivateKey, PublicKey};
use uuid::Uuid;

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
            if global.get_bucket_by_origin(origin).is_some() {
                return Err(anyhow!("Bucket already exists at this origin"));
            }

            // Initialize in the configs
            let mut config = global.new_bucket(origin).await?;

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
        } => extract::pipeline(&mut global, &bucket_specifier, &output).await, // List all Buckets tracked remotely and locally
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
                .map(|v| format!("id:\t{}\nresponse:\t{}", bucket_id, v))
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
            match subcommand {
                MetadataSubCommand::Read {
                    bucket_specifier,
                    metadata_id,
                } => {
                    // Get Bucket config
                    let config = global.get_bucket_by_specifier(&bucket_specifier)?;
                    // If we can get the metadata
                    if let Some(remote_id) = config.remote_id {
                        Metadata::read(remote_id, metadata_id, &mut client)
                            .await
                            .map(|metadata| format!("{:?}", metadata))
                            .map_err(TombError::client_error)
                    } else {
                        Err(TombError::anyhow_error(anyhow!(
                            "Conffig has no remote id!"
                        )))
                    }
                }
                MetadataSubCommand::Push(bucket_specifier) => {
                    // Get info
                    let wrapping_key = global.wrapping_key().await?;
                    let config = global.get_bucket_by_specifier(&bucket_specifier)?;
                    let fs = FsMetadata::unlock(&wrapping_key, &config.metadata).await?;
                    let valid_keys = fs.share_manager.recipients();

                    let expected_data_size = compute_directory_size(&config.metadata.path)? as u64;
                    let bucket_id = config.remote_id.expect("no remote id");
                    let root_cid = config.metadata.get_root().expect("no root cid").to_string();
                    let metadata_stream = tokio::fs::File::open(&config.metadata.path).await?;
                    // Push the Metadata
                    Metadata::push(
                        bucket_id,
                        root_cid,
                        expected_data_size,
                        metadata_stream,
                        valid_keys,
                        &mut client,
                    )
                    .await
                    .map(|(metadata, storage_ticket)| {
                        let mut info = format!("\t{}", metadata);
                        if let Some(storage_ticket) = storage_ticket {
                            info.push_str(&format!("\n\n\t{}", storage_ticket))
                        }
                        info
                    })
                    .map_err(TombError::client_error)
                }
                MetadataSubCommand::ReadCurrent(bucket_specifier) => {
                    let config = global.get_bucket_by_specifier(&bucket_specifier)?;
                    let bucket_id = config.remote_id.expect("no remote id");
                    Metadata::read_current(bucket_id, &mut client).await
                    .map(|metadata| {
                        format!("{:?}", metadata)
                    })
                    .map_err(TombError::client_error)
                },
                MetadataSubCommand::List(bucket_specifier) => {
                    let config = global.get_bucket_by_specifier(&bucket_specifier)?;
                    let bucket_id = config.remote_id.expect("no remote id");
                    Metadata::read_all(bucket_id, &mut client).await
                    .map(|metadatas| {
                        metadatas
                        .iter()
                        .fold("<< METADATAS >>".to_string(), |acc, metadata| {
                            format!("{}{}", acc, metadata)
                        })
                    })
                    .map_err(TombError::client_error)
                },
                MetadataSubCommand::Pull { bucket_specifier, metadata_id } => {
                    let config = global.get_bucket_by_specifier(&bucket_specifier)?;
                    let bucket_id = config.remote_id.expect("no remote id");
                    let metadata = Metadata::read(bucket_id, metadata_id, &mut client).await?;
                    let mut byte_stream = metadata.pull(&mut client).await?;
                    let mut file = tokio::fs::File::create(&config.metadata.path).await?;

                    println!("starting to download metadata...");

                    while let Some(chunk) = byte_stream.next().await {
                        tokio::io::copy(&mut chunk?.as_ref(), &mut file).await?;
                    }

                    Ok(format!("successfully downloaded metadata"))
                },
                MetadataSubCommand::Snapshot { bucket_specifier, metadata_id } => {
                    let config = global.get_bucket_by_specifier(&bucket_specifier)?;
                    let bucket_id = config.remote_id.expect("no remote id");
                    let metadata = Metadata::read(bucket_id, metadata_id, &mut client).await?;
                    
                    metadata.snapshot(&mut client).await.map(|snapshot| {
                        format!("{:?}", snapshot)
                    }).map_err(TombError::client_error)
                },
            }
        }
        // Bucket Key Management
        BucketsSubCommand::Keys { subcommand } => match subcommand {
            // List Keys
            KeySubCommand::List(bucket_specifier) => {
                BucketKey::read_all(global.get_bucket_id(&bucket_specifier)?, &mut client)
                    .await
                    .map(|keys| {
                        keys.iter().fold(String::new(), |acc, key| {
                            format!("{}\n\n{}", acc, format!("{}", key))
                        })
                    })
                    .map_err(TombError::client_error)
            }
            // Create a new key
            KeySubCommand::Create(bucket_specifier) => {
                let private_key = EcEncryptionKey::generate().await?;
                let public_key = private_key.public_key()?;
                let pem = String::from_utf8(public_key.export().await?)?;
                let bucket_id = global.get_bucket_id(&bucket_specifier)?;
                BucketKey::create(bucket_id, pem, &mut client)
                    .await
                    .map(|key| format!("{}", key))
                    .map_err(TombError::client_error)
            }
            // Delete an already approved key
            KeySubCommand::Delete(ks) => {
                let (bucket_id, id) = get_key_ids(&global, &ks)?;
                BucketKey::delete_by_id(bucket_id, id, &mut client)
                    .await
                    .map(|id| format!("deleted key!\nid:\t{}", id))
                    .map_err(TombError::client_error)
            }
            // Get info about a Key
            KeySubCommand::Info(ks) => {
                let (bucket_id, id) = get_key_ids(&global, &ks)?;
                BucketKey::read(bucket_id, id, &mut client)
                    .await
                    .map(|key| format!("{}", key))
                    .map_err(TombError::client_error)
            }
            // Reject a Key pending approval
            KeySubCommand::Reject(ks) => {
                let (bucket_id, id) = get_key_ids(&global, &ks)?;
                BucketKey::reject(bucket_id, id, &mut client)
                    .await
                    .map(|id| format!("rejected key!\nid:\t{}", id))
                    .map_err(TombError::client_error)
            }
        },
    };

    // Save the Client
    global.save_client(client).await?;
    global.to_disk()?;

    // Return
    result.map_err(anyhow::Error::new)
}

fn get_key_ids(global: &GlobalConfig, key_specifier: &KeySpecifier) -> Result<(Uuid, Uuid)> {
    Ok((
        global.get_bucket_id(&key_specifier.bucket_specifier)?,
        key_specifier.key_id,
    ))
}
