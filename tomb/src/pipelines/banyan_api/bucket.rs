use std::{env::current_dir, path::PathBuf};

use crate::{cli::command::*, pipelines::configure, types::config::globalconfig::GlobalConfig};
use anyhow::{anyhow, Result};
use tomb_common::banyan_api::models::{
    bucket::{Bucket, BucketType, StorageClass},
    bucket_key::BucketKey,
};
use tomb_crypt::prelude::{EcEncryptionKey, PrivateKey, PublicKey};
use uuid::Uuid;

fn get_bucket_id(global: &GlobalConfig, origin: Option<PathBuf>) -> Result<Uuid> {
    let origin = origin.unwrap_or(current_dir()?);
    if let Some(bucket) = global.get_bucket_by_origin(&origin) && let Some(id) = bucket.id {
        Ok(id)
    } else {
        Err(anyhow!("This bucket is not yet configured"))
    }
}

fn get_bucket_info(global: &GlobalConfig, bucket: &Bucket) -> String {
    let remote_info = format!(
        "name:\t\t{}\nid:\t\t{}\ntype:\t\t{}\nstorage class:\t{}",
        bucket.name, bucket.id, bucket.r#type, bucket.storage_class
    );
    let location = if let Some(config) = global.get_bucket_by_id(&bucket.id) {
        format!("{}", config.origin.display())
    } else {
        "unknown".to_string()
    };
    let local_info = format!("local path:\t{}", location);

    format!("| BUCKET INFO |\n{}\n{}", remote_info, local_info)
}

pub async fn pipeline(command: BucketSubCommand) -> Result<String> {
    // Grab global config
    let mut global = GlobalConfig::from_disk().await?;
    // Obtain the Client
    let mut client = global.get_client().await?;

    // Process the command
    let result = match command {
        BucketSubCommand::Create { origin, name } => {
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
                config.id = Some(bucket.id);
                // Update the config globally
                global
                    .update_config(&config)
                    .expect("unable to update config to include local path");
                // Return
                format!("new bucket: {:?}\nnew bucket key: {}", bucket, key)
            })
        }
        BucketSubCommand::List => Bucket::read_all(&mut client).await.map(|buckets| {
            buckets.iter().fold(String::new(), |acc, bucket| {
                format!("{}\n\n{}", acc, get_bucket_info(&global, bucket))
            })
        }),

        BucketSubCommand::Modify { origin, subcommand } => {
            // Get the bucket ID
            let bucket_id = get_bucket_id(&global, origin)?;

            match subcommand {
                ModifyBucketSubCommand::Push => todo!(),
                ModifyBucketSubCommand::Pull => todo!(),
                ModifyBucketSubCommand::Delete => Bucket::delete_by_id(&mut client, bucket_id)
                    .await
                    .map(|v| format!("bucket {}: {}", bucket_id, v)),
                ModifyBucketSubCommand::Info => Bucket::read(&mut client, bucket_id)
                    .await
                    .map(|bucket| format!("{}", get_bucket_info(&global, &bucket))),
                ModifyBucketSubCommand::Usage => Bucket::read(&mut client, bucket_id)
                    .await?
                    .usage(&mut client)
                    .await
                    .map(|v| format!("bucket {} usage: {}", bucket_id, v)),
                ModifyBucketSubCommand::Keys { subcommand } => match subcommand {
                    KeySubCommand::List => {
                        BucketKey::read_all(bucket_id, &mut client)
                            .await
                            .map(|keys| {
                                keys.iter()
                                    .fold(String::new(), |acc, key| format!("{}\n{}", acc, key))
                            })
                    }
                    KeySubCommand::Create => {
                        let private_key = EcEncryptionKey::generate().await?;
                        let public_key = private_key.public_key()?;
                        let pem = String::from_utf8(public_key.export().await?)?;
                        BucketKey::create(bucket_id, pem, &mut client)
                            .await
                            .map(|v| format!("|KEY INFO|\n{}", v))
                    }
                    KeySubCommand::Modify { id, subcommand } => match subcommand {
                        ModifyKeySubCommand::Delete => {
                            BucketKey::delete_by_id(bucket_id, id, &mut client)
                                .await
                                .map(|v| format!("key {}:\n{}", id, v))
                        }
                        ModifyKeySubCommand::Info => BucketKey::read(bucket_id, id, &mut client)
                            .await
                            .map(|v| format!("|KEY INFO|\n\n{}", v)),
                        ModifyKeySubCommand::Approve => todo!(),
                        ModifyKeySubCommand::Reject => todo!(),
                    },
                },
            }
        }
    };

    // Save the Client
    global.save_client(client).await?;

    // Return
    result.map_err(anyhow::Error::new)
}
