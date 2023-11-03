use crate::{
    pipelines::error::TombError,
    types::config::{bucket::OmniBucket, globalconfig::GlobalConfig},
};

use super::{super::specifiers::*, RunnableCommand};
use async_trait::async_trait;
use clap::Subcommand;
use colored::Colorize;
use tomb_common::banyan_api::{client::Client, error::ClientError, models::bucket_key::BucketKey};
use tomb_crypt::{
    hex_fingerprint,
    prelude::{PrivateKey, PublicKey},
};
use uuid::Uuid;

/// Subcommand for Drive Keys
#[derive(Subcommand, Clone, Debug)]
pub enum KeyCommand {
    /// Request Access to a Drive if you dont already have it
    RequestAccess(DriveSpecifier),
    /// List all Keys in a Drive
    Ls(DriveSpecifier),
    /// Get information about an individual Drive Key
    Info(KeySpecifier),
    /// Delete a given Key
    Delete(KeySpecifier),
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
            KeyCommand::RequestAccess(drive_specifier) => {
                let private_key = global.wrapping_key().await?;
                let public_key = private_key
                    .public_key()
                    .map_err(ClientError::crypto_error)?;
                // Compute PEM
                let fingerprint = hex_fingerprint(&public_key.fingerprint().await?.to_vec());
                let pem = String::from_utf8(
                    public_key
                        .export()
                        .await
                        .map_err(ClientError::crypto_error)?,
                )
                .unwrap();

                // Get Drive
                let omni = OmniBucket::from_specifier(global, client, &drive_specifier).await;
                if let Ok(id) = omni.get_id() {
                    let existing_keys = BucketKey::read_all(id, client).await?;
                    if let Some(existing_key) = existing_keys
                        .iter()
                        .find(|key| key.fingerprint == fingerprint)
                    {
                        info!("\n{}\n", existing_key.context_fmt(&fingerprint));
                        Err(TombError::custom_error(
                            "You've already requested access on this Bucket!",
                        ))
                    } else {
                        BucketKey::create(id, pem, client)
                            .await
                            .map(|key| format!("\n{}", key))
                            .map_err(TombError::client_error)
                    }
                } else {
                    Err(TombError::custom_error(
                        "Cannot request key access on a Bucket with no known remote correlate.",
                    ))
                }
            }
            KeyCommand::Ls(drive_specifier) => {
                let omni = OmniBucket::from_specifier(global, client, &drive_specifier).await;
                let id = omni.get_id().unwrap();
                let my_fingerprint = hex_fingerprint(
                    &global
                        .wrapping_key()
                        .await?
                        .public_key()?
                        .fingerprint()
                        .await?
                        .to_vec(),
                );
                BucketKey::read_all(id, client)
                    .await
                    .map(|keys| {
                        keys.iter().fold(String::new(), |acc, key| {
                            format!("{}\n\n{}", acc, key.context_fmt(&my_fingerprint))
                        })
                    })
                    .map_err(TombError::client_error)
            }
            KeyCommand::Info(ks) => {
                let (bucket_id, id) = get_key_info(client, global, &ks).await?;
                let my_fingerprint = hex_fingerprint(
                    &global
                        .wrapping_key()
                        .await?
                        .public_key()?
                        .fingerprint()
                        .await?
                        .to_vec(),
                );
                BucketKey::read(bucket_id, id, client)
                    .await
                    .map(|key| key.context_fmt(&my_fingerprint))
                    .map_err(TombError::client_error)
            }
            KeyCommand::Delete(ks) => {
                let (bucket_id, id) = get_key_info(client, global, &ks).await?;
                BucketKey::delete_by_id(bucket_id, id, client)
                    .await
                    .map(|id| format!("<< DELETED KEY SUCCESSFULLY >>\nid:\t{}", id))
                    .map_err(TombError::client_error)
            }
            KeyCommand::Reject(ks) => {
                let (bucket_id, id) = get_key_info(client, global, &ks).await?;
                BucketKey::reject(bucket_id, id, client)
                    .await
                    .map(|_| format!("{}", "<< REJECTED KEY SUCCESSFULLY >>".green()))
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
    let bucket_id = OmniBucket::from_specifier(global, client, &key_specifier.drive_specifier)
        .await
        .get_id()
        .unwrap();
    let all_keys = BucketKey::read_all(bucket_id, client).await?;
    let key_index = all_keys
        .iter()
        .position(|key| key.fingerprint == key_specifier.fingerprint)
        .unwrap();
    let key = all_keys[key_index].clone();

    Ok((bucket_id, key.id))
}
