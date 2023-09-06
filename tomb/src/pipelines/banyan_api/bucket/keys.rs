use anyhow::anyhow;
use tomb_common::{
    banyan_api::{client::Client, error::ClientError, models::bucket_key::BucketKey},
    metadata::FsMetadata,
};
use tomb_crypt::{
    prelude::{EcEncryptionKey, PrivateKey, PublicKey},
    pretty_fingerprint,
};
use uuid::Uuid;

use crate::{
    cli::command::{KeySpecifier, KeySubCommand},
    pipelines::error::TombError,
    types::config::globalconfig::GlobalConfig,
};

/// Pipeline for processing BucketKey commands
pub(crate) async fn pipeline(
    global: &GlobalConfig,
    client: &mut Client,
    command: KeySubCommand,
) -> Result<String, TombError> {
    match command {
        // List Keys
        KeySubCommand::List(bucket_specifier) => {
            BucketKey::read_all(global.get_bucket_id(&bucket_specifier)?, client)
                .await
                .map(|keys| {
                    keys.iter()
                        .fold(String::new(), |acc, key| format!("{}\n\n{}", acc, key))
                })
                .map_err(TombError::client_error)
        }
        // Create a new key
        KeySubCommand::Create(bucket_specifier) => {
            // let all_keys = BucketKey::read_all(global.get_bucket_id(&bucket_specifier)?, client).await?;
            let public_key = global
                .wrapping_key()
                .await?
                .public_key()
                .map_err(ClientError::crypto_error)?;
            // Compute PEM
            let pem = String::from_utf8(
                public_key
                    .export()
                    .await
                    .map_err(ClientError::crypto_error)?,
            )
            .unwrap();
            // // If the current fingerprint is already in the
            // if all_keys.iter().position(|key| key.pem == pem).is_some() {
            //     return Err(TombError::anyhow_error(anyhow!("this device key is already used").into()))
            // }

            // Get Bucket
            let bucket = global.get_bucket_by_specifier(&bucket_specifier)?;
            let mut fs =
                FsMetadata::unlock(&global.wrapping_key().await?, &bucket.metadata).await?;
            fs.share_with(&public_key, &bucket.metadata).await?;
            fs.save(&bucket.metadata, &bucket.content).await?;

            if let Some(remote_id) = bucket.remote_id {
                BucketKey::create(remote_id, pem, client)
                    .await
                    .map(|key| format!("{}", key))
                    .map_err(TombError::client_error)
            } else {
                Ok(format!("added key to bucket locally"))
            }
        }
        // Delete an already approved key
        KeySubCommand::Delete(ks) => {
            let (bucket_id, id) = get_key_ids(global, &ks)?;
            BucketKey::delete_by_id(bucket_id, id, client)
                .await
                .map(|id| format!("deleted key!\nid:\t{}", id))
                .map_err(TombError::client_error)
        }
        // Get info about a Key
        KeySubCommand::Info(ks) => {
            let (bucket_id, id) = get_key_ids(global, &ks)?;
            BucketKey::read(bucket_id, id, client)
                .await
                .map(|key| format!("{}", key))
                .map_err(TombError::client_error)
        }
        // Reject a Key pending approval
        KeySubCommand::Reject(ks) => {
            let (bucket_id, id) = get_key_ids(global, &ks)?;
            BucketKey::reject(bucket_id, id, client)
                .await
                .map(|id| format!("rejected key!\nid:\t{}", id))
                .map_err(TombError::client_error)
        }
    }
}

fn get_key_ids(
    global: &GlobalConfig,
    key_specifier: &KeySpecifier,
) -> anyhow::Result<(Uuid, Uuid)> {
    Ok((
        global.get_bucket_id(&key_specifier.bucket_specifier)?,
        key_specifier.key_id,
    ))
}
