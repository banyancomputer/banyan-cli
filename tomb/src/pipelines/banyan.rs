use std::{path::PathBuf, env::current_dir, fs::File, str::FromStr};

use crate::{cli::command::*, types::config::globalconfig::GlobalConfig};
use anyhow::{anyhow, Result};
use tomb_common::banyan_api::{
    client::{Client, Credentials, self},
    error::ClientError,
    models::{
        account::Account,
        bucket::{Bucket, BucketType, StorageClass},
        bucket_key::BucketKey,
    },
    requests::{auth::fake_account::create::*, buckets::read::ReadAllBuckets},
};
use tomb_crypt::prelude::{EcEncryptionKey, EcSignatureKey, PrivateKey, PublicKey};
use uuid::Uuid;

use super::configure;

async fn get_bucket_id(global: &GlobalConfig, origin: Option<PathBuf>) -> Result<Uuid> {
    let origin = origin.unwrap_or(current_dir()?);
    if let Some(bucket) = global.get_bucket(&origin) && let Some(id) = bucket.id {
        Ok(id)
    } else {
        Err(anyhow!("This bucket is not yet configured"))
    }
} 

pub async fn auth(command: AuthSubCommand) -> Result<String> {
    // Grab global config
    let mut global = GlobalConfig::from_disk().await?;
    // Obtain the Client
    let mut client = global.get_client().await?;

    // Process the command
    let result = match command {
        AuthSubCommand::Register => {
            // Create local keys
            let api_key = EcSignatureKey::generate().await?;
            let public_api_key = api_key.public_key()?;
            let public_api_key_pem = String::from_utf8(public_api_key.export().await?)?;
            // Associate the key material with the backend
            let response: CreateAccountResponse = client
                .call(CreateAccount {
                    device_api_key_pem: public_api_key_pem,
                })
                .await?;
            client.with_credentials(Credentials {
                account_id: response.id,
                signing_key: api_key.clone(),
            });

            Ok(format!("created account with id: {}", response.id))
        },
        
        AuthSubCommand::Login => todo!(),
        AuthSubCommand::WhoAmI => Account::who_am_i(&mut client)
            .await
            .map(|v| format!("account: {}", v.id)),
        AuthSubCommand::Usage => Account::usage(&mut client)
            .await
            .map(|v| format!("usage: {}", v)),
        AuthSubCommand::Limit => Account::usage_limit(&mut client)
            .await
            .map(|v| format!("usage limit: {}", v)),
    };

    // Save the Client
    global.save_client(client).await?;

    // Return 
    result.map_err(anyhow::Error::new)
}

pub async fn bucket(command: BucketSubCommand) -> Result<String> {
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
            configure::init(&origin.unwrap_or(current_dir()?)).await?;

            // Initialize on the remote endpoint
            Bucket::create(
                name,
                pem,
                BucketType::Interactive,
                StorageClass::Hot,
                &mut client,
            )
            .await
            .map(|(b, k)| format!("new bucket: {:?}\nnew bucket key: {}", b, k))
        },
        BucketSubCommand::List => Bucket::read_all(&mut client)
            .await
            .map(|v| format!("buckets: {:?}", v)),
        BucketSubCommand::Modify { origin, subcommand } => {
            // Get the bucket ID
            let bucket_id = get_bucket_id(&global, origin).await?;

            match subcommand {
                ModifyBucketSubCommand::Push => todo!(),
                ModifyBucketSubCommand::Pull => todo!(),
                ModifyBucketSubCommand::Delete => Bucket::delete_by_id(&mut client, bucket_id)
                    .await
                    .map(|v| format!("bucket {}: {}", bucket_id, v)),
                ModifyBucketSubCommand::Info => Bucket::read(&mut client, bucket_id)
                    .await
                    .map(|v| format!("bucket info: {:?}", v)),
                ModifyBucketSubCommand::Usage => Bucket::read(&mut client, bucket_id)
                    .await?
                    .usage(&mut client)
                    .await
                    .map(|v| format!("bucket {} usage: {}", bucket_id, v)),
                ModifyBucketSubCommand::Keys { subcommand } => match subcommand {
                    KeySubCommand::List => BucketKey::read_all(bucket_id, &mut client)
                    .await
                    .map(|v| format!("keys: {:?}", v)),
                    KeySubCommand::Create => {
                        let private_key = EcEncryptionKey::generate().await?;
                        let public_key = private_key.public_key()?;
                        let pem = String::from_utf8(public_key.export().await?)?;
                        BucketKey::create(bucket_id, pem, &mut client)
                            .await
                            .map(|v| format!("bucket key created: {}", v))
                    },
                    KeySubCommand::Modify { id, subcommand } => match subcommand {
                        ModifyKeySubCommand::Delete => BucketKey::delete_by_id(bucket_id, id, &mut client)
                            .await
                            .map(|v| format!("key {}: {}", id, v)),
                        ModifyKeySubCommand::Info => BucketKey::read(bucket_id, id, &mut client)
                        .await
                        .map(|v| format!("key: {}", v)),
                        ModifyKeySubCommand::Approve => todo!(),
                        ModifyKeySubCommand::Reject => todo!(),
                    },
                }
            }
        },
    };

    // Save the Client
    global.save_client(client).await?;

    // Return 
    result.map_err(anyhow::Error::new)
}

