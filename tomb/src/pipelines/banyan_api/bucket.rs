use std::env::current_dir;

use crate::{cli::command::*, types::config::globalconfig::GlobalConfig};
use anyhow::{anyhow, Result};
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
    let result = match command {
        BucketsSubCommand::Create { name, origin } => {
            let private_key = EcEncryptionKey::generate().await?;
            let public_key = private_key.public_key()?;
            let pem = String::from_utf8(public_key.export().await?)?;

            // Initialize in the configs
            let mut config = global.new_bucket(&origin.unwrap_or(current_dir()?)).await?;

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
                format!("new bucket: {:?}\nnew bucket key: {}", bucket, key)
            })
        }
        BucketsSubCommand::List => Bucket::read_all(&mut client).await.map(|buckets| {
            buckets.iter().fold(String::new(), |acc, bucket| {
                format!("{}\n\n{}", acc, get_bucket_string(&global, bucket))
            })
        }),
        BucketsSubCommand::Push(_) => todo!(),
        BucketsSubCommand::Pull(_) => todo!(),
        BucketsSubCommand::Delete(bs) => {
            let bucket_id = get_bucket_id(&global, &bs)?;
            Bucket::delete_by_id(&mut client, bucket_id)
                .await
                .map(|v| format!("id:\t{}\nresponse:\t{}", bucket_id, v))
        }
        BucketsSubCommand::Info(bs) => Bucket::read(&mut client, get_bucket_id(&global, &bs)?)
            .await
            .map(|bucket| get_bucket_string(&global, &bucket)),
        BucketsSubCommand::Usage(bs) => {
            let bucket_id = get_bucket_id(&global, &bs)?;
            Bucket::read(&mut client, bucket_id)
                .await?
                .usage(&mut client)
                .await
                .map(|v| format!("id:\t{}\nusage:\t{}", bucket_id, v))
        }
        BucketsSubCommand::Keys { subcommand } => match subcommand {
            KeySubCommand::List(bs) => {
                BucketKey::read_all(get_bucket_id(&global, &bs)?, &mut client)
                    .await
                    .map(|keys| {
                        keys.iter().fold(String::new(), |acc, key| {
                            format!("{}\n\n{}", acc, get_key_string(key))
                        })
                    })
            }
            KeySubCommand::Create(bs) => {
                let private_key = EcEncryptionKey::generate().await?;
                let public_key = private_key.public_key()?;
                let pem = String::from_utf8(public_key.export().await?)?;
                let bucket_id = get_bucket_id(&global, &bs)?;
                BucketKey::create(bucket_id, pem, &mut client)
                    .await
                    .map(|key| get_key_string(&key))
            }
            KeySubCommand::Delete(ks) => {
                let (bucket_id, id) = get_key_ids(&global, &ks)?;
                BucketKey::delete_by_id(bucket_id, id, &mut client)
                    .await
                    .map(|v| format!("key {}:\n{}", id, v))
            }
            KeySubCommand::Info(ks) => {
                let (bucket_id, id) = get_key_ids(&global, &ks)?;
                BucketKey::read(bucket_id, id, &mut client)
                    .await
                    .map(|key| get_key_string(&key))
            }
            KeySubCommand::Approve(ks) => {
                let (bucket_id, id) = get_key_ids(&global, &ks)?;
                BucketKey::approve(bucket_id, id, &mut client)
                    .await
                    .map(|key| get_key_string(&key))
            },
            KeySubCommand::Reject(_) => todo!(),
        },
    };

    // Save the Client
    global.save_client(client).await?;

    // Return
    result.map_err(anyhow::Error::new)
}

fn get_bucket_id(global: &GlobalConfig, bucket_specifier: &BucketSpecifier) -> Result<Uuid> {
    // If we already have the ID
    if let Some(id) = bucket_specifier.bucket_id {
        Ok(id)
    } else {
        // Grab an Origin
        let origin = bucket_specifier.origin.clone().unwrap_or(current_dir()?);
        // Find a BucketConfig at this origin and expect it has an ID saved as well
        if let Some(bucket) = global.get_bucket_by_origin(&origin) && let Some(id) = bucket.remote_id {
            Ok(id)
        } else {
            Err(anyhow!("This directory is not configured as a Bucket"))
        }
    }
}

fn get_key_ids(global: &GlobalConfig, key_specifier: &KeySpecifier) -> Result<(Uuid, Uuid)> {
    Ok((
        get_bucket_id(global, &key_specifier.bucket)?,
        key_specifier.key_id,
    ))
}

fn get_bucket_string(global: &GlobalConfig, bucket: &Bucket) -> String {
    let remote_info = format!(
        "name:\t\t{}\nid:\t\t{}\ntype:\t\t{}\nstorage class:\t{}",
        bucket.name, bucket.id, bucket.r#type, bucket.storage_class
    );
    let location = if let Some(config) = global.get_bucket_by_remote_id(&bucket.id) {
        format!("{}", config.origin.display())
    } else {
        "unknown".to_string()
    };
    let local_info = format!("local path:\t{}", location);

    format!("| BUCKET INFO |\n{}\n{}", remote_info, local_info)
}

fn get_key_string(key: &BucketKey) -> String {
    format!("| KEY INFO |\n{}", key)
}