use std::env::current_dir;

use crate::{
    cli::command::*,
    pipelines::{bundle, error::PipelineError, extract},
    types::config::globalconfig::GlobalConfig,
};
use anyhow::{Result, anyhow};
use tomb_common::banyan_api::models::{
    bucket::{Bucket, BucketType, StorageClass},
    bucket_key::BucketKey,
};
use tomb_crypt::prelude::{EcEncryptionKey, PrivateKey, PublicKey};
use uuid::Uuid;

/// Handle Bucket management both locally and remotely based on CLI input
pub async fn pipeline(command: BucketsSubCommand) -> Result<String> {
    // Grab global config
    let mut global = GlobalConfig::from_disk().await?;
    // Obtain the Client
    let mut client = global.get_client().await?;

    // Process the command
    let result: Result<String, PipelineError> = match command {
        // Create a new Bucket. This attempts to create the Bucket both locally and remotely, but settles for a simple local creation if remote permissions fail
        BucketsSubCommand::Create { name, origin } => {
            let private_key = EcEncryptionKey::generate().await?;
            let public_key = private_key.public_key()?;
            let pem = String::from_utf8(public_key.export().await?)?;

            let origin = &origin.unwrap_or(current_dir()?);

            if global.get_bucket_by_origin(origin).is_some() {
                return Err(anyhow!("Bucket already exists at this origin").into())
            }
            
            // Initialize in the configs
            let mut config = global.new_bucket(origin).await?;

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
                    .expect("unable to update config to include local path");
                // Return
                format!("<<NEW BUCKET CREATED >>\n{}\n{}\n{}", bucket, config, key)
            })
            .map_err(PipelineError::client_error)
        }
        // List all Buckets tracked remotely and locally
        BucketsSubCommand::List => Bucket::read_all(&mut client)
            .await
            .map(|buckets| {
                let remote = buckets.iter().fold(String::new(), |acc, bucket| {
                    format!("{}\n\n{}", acc, bucket)
                });

                let local = global.buckets.iter().fold(String::new(), |acc, bucket| {
                    format!("{}\n\n{}", acc, bucket)
                });
                
                format!("<< REMOTE BUCKETS >>{}\n\n<< LOCAL BUCKETS >>{}", remote, local)
            })
            .map_err(PipelineError::client_error),
        BucketsSubCommand::Push(_) => todo!(),
        BucketsSubCommand::Pull(_) => todo!(),
        BucketsSubCommand::Bundle {
            bucket_specifier,
            follow_links,
        } => bundle::pipeline(&bucket_specifier, follow_links).await,
        BucketsSubCommand::Extract {
            bucket_specifier,
            output,
        } => extract::pipeline(&bucket_specifier, &output).await,
        BucketsSubCommand::Delete(bs) => {
            let bucket_id = global.get_bucket_id(&bs)?;
            Bucket::delete_by_id(&mut client, bucket_id)
                .await
                .map(|v| format!("id:\t{}\nresponse:\t{}", bucket_id, v))
                .map_err(PipelineError::client_error)
        }
        BucketsSubCommand::Info(bs) => Bucket::read(&mut client, global.get_bucket_id(&bs)?)
            .await
            .map(|bucket| {
                let config = global.get_bucket_by_remote_id(&bucket.id).unwrap();
                format!("{}{}", bucket, config)
            })
            .map_err(PipelineError::client_error),
        BucketsSubCommand::Usage(bs) => {
            let bucket_id = global.get_bucket_id(&bs)?;
            Bucket::read(&mut client, bucket_id)
                .await?
                .usage(&mut client)
                .await
                .map(|v| format!("id:\t{}\nusage:\t{}", bucket_id, v))
                .map_err(PipelineError::client_error)
        }
        BucketsSubCommand::Keys { subcommand } => match subcommand {
            KeySubCommand::List(bs) => BucketKey::read_all(global.get_bucket_id(&bs)?, &mut client)
                .await
                .map(|keys| {
                    keys.iter().fold(String::new(), |acc, key| {
                        format!("{}\n\n{}", acc, format!("{}", key))
                    })
                })
                .map_err(PipelineError::client_error),
            KeySubCommand::Create(bs) => {
                let private_key = EcEncryptionKey::generate().await?;
                let public_key = private_key.public_key()?;
                let pem = String::from_utf8(public_key.export().await?)?;
                let bucket_id = global.get_bucket_id(&bs)?;
                BucketKey::create(bucket_id, pem, &mut client)
                    .await
                    .map(|key| format!("{}", key))
                    .map_err(PipelineError::client_error)
            }
            KeySubCommand::Delete(ks) => {
                let (bucket_id, id) = get_key_ids(&global, &ks)?;
                BucketKey::delete_by_id(bucket_id, id, &mut client)
                    .await
                    .map(|id| format!("deleted key!\nid:\t{}", id))
                    .map_err(PipelineError::client_error)
            }
            KeySubCommand::Info(ks) => {
                let (bucket_id, id) = get_key_ids(&global, &ks)?;
                BucketKey::read(bucket_id, id, &mut client)
                    .await
                    .map(|key| format!("{}", key))
                    .map_err(PipelineError::client_error)
            }
            KeySubCommand::Reject(ks) => {
                let (bucket_id, id) = get_key_ids(&global, &ks)?;
                BucketKey::reject(bucket_id, id, &mut client)
                    .await
                    .map(|id| format!("rejected key!\nid:\t{}", id))
                    .map_err(PipelineError::client_error)
            }
        },
    };

    // Save the Client
    global.save_client(client).await?;

    // Return
    result.map_err(anyhow::Error::new)
}

fn get_key_ids(global: &GlobalConfig, key_specifier: &KeySpecifier) -> Result<(Uuid, Uuid)> {
    Ok((
        global.get_bucket_id(&key_specifier.bucket)?,
        key_specifier.key_id,
    ))
}