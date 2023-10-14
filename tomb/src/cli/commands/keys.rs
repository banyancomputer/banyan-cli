use crate::{pipelines::error::TombError, types::config::globalconfig::GlobalConfig};

use super::{super::specifiers::*, RunnableCommand};
use async_trait::async_trait;
use clap::Subcommand;
use tomb_common::{
    banyan_api::{client::Client, error::ClientError, models::bucket_key::BucketKey},
    metadata::FsMetadata,
};
use tomb_crypt::prelude::{PrivateKey, PublicKey};
use uuid::Uuid;

/// Subcommand for Bucket Keys
#[derive(Subcommand, Clone, Debug)]
pub enum KeyCommand {
    /// List all Keys in a Bucket
    List(BucketSpecifier),
    /// Request Access to a Bucket if you dont already have it
    RequestAccess(BucketSpecifier),
    /// Delete a given Key
    Delete(KeySpecifier),
    /// List the keys persisted by the remote endpoint
    Info(KeySpecifier),
    /// Reject or remove a key and sync that witht the remote endpoint
    Reject(KeySpecifier),
}

#[async_trait(?Send)]
impl RunnableCommand<TombError> for KeyCommand {
    async fn run_internal(
        self,
        global: &mut GlobalConfig,
        client: &mut Client,
    ) -> Result<String, TombError> {
        match self {
            // List Keys
            KeyCommand::List(bucket_specifier) => {
                BucketKey::read_all(global.get_bucket_id(&bucket_specifier)?, client)
                    .await
                    .map(|keys| {
                        keys.iter()
                            .fold(String::new(), |acc, key| format!("{}\n\n{}", acc, key))
                    })
                    .map_err(TombError::client_error)
            }
            // Request access to a bucket
            KeyCommand::RequestAccess(bucket_specifier) => {
                let private_key = global.wrapping_key().await?;
                let public_key = private_key
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

                // Get Bucket
                let bucket = global.get_bucket_by_specifier(&bucket_specifier)?;
                let mut fs = FsMetadata::unlock(&private_key, &bucket.metadata).await?;
                fs.share_with(&public_key, &bucket.metadata).await?;
                fs.save(&bucket.metadata, &bucket.metadata).await?;

                if let Some(remote_id) = bucket.remote_id {
                    BucketKey::create(remote_id, pem, client)
                        .await
                        .map(|key| format!("{}", key))
                        .map_err(TombError::client_error)
                } else {
                    Ok("added key to bucket locally".to_string())
                }
            }
            // Delete an already approved key
            KeyCommand::Delete(ks) => {
                let (bucket_id, id) = get_key_info(client, global, &ks).await?;
                BucketKey::delete_by_id(bucket_id, id, client)
                    .await
                    .map(|id| format!("deleted key!\nid:\t{}", id))
                    .map_err(TombError::client_error)
            }
            // Get info about a Key
            KeyCommand::Info(ks) => {
                let (bucket_id, id) = get_key_info(client, global, &ks).await?;
                BucketKey::read(bucket_id, id, client)
                    .await
                    .map(|key| format!("{}", key))
                    .map_err(TombError::client_error)
            }
            // Reject a Key pending approval
            KeyCommand::Reject(ks) => {
                let (bucket_id, id) = get_key_info(client, global, &ks).await?;
                BucketKey::reject(bucket_id, id, client)
                    .await
                    .map(|id| format!("rejected key!\nid:\t{}", id))
                    .map_err(TombError::client_error)
            }
        }
    }
}

async fn get_key_info(
    client: &mut Client,
    global: &GlobalConfig,
    key_specifier: &KeySpecifier,
) -> anyhow::Result<(Uuid, Uuid)> {
    let bucket_id = global.get_bucket_id(&key_specifier.bucket_specifier)?;
    let all_keys = BucketKey::read_all(bucket_id, client).await?;
    let key_index = all_keys
        .iter()
        .position(|key| key.fingerprint == key_specifier.fingerprint)
        .unwrap();
    let key = all_keys[key_index].clone();

    Ok((bucket_id, key.id))
}
