use crate::{
    api::{client::Client, error::ApiError, models::bucket_key::BucketKey},
    cli::CliError,
    native::{
        configuration::{bucket::OmniBucket, globalconfig::GlobalConfig, SyncError},
        operations::NativeError,
    },
};

use super::{
    super::specifiers::{DriveSpecifier, KeySpecifier},
    RunnableCommand,
};
use async_trait::async_trait;
use clap::Subcommand;
use colored::Colorize;
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
impl RunnableCommand<CliError> for KeyCommand {
    async fn run_internal(
        self,
        global: &mut GlobalConfig,
        client: &mut Client,
    ) -> Result<String, CliError> {
        match self {
            KeyCommand::RequestAccess(drive_specifier) => {
                let private_key = global.wrapping_key().await?;
                let public_key = private_key.public_key().map_err(ApiError::crypto)?;
                // Compute PEM
                let fingerprint = hex_fingerprint(&public_key.fingerprint().await?.to_vec());
                let pem = String::from_utf8(public_key.export().await.map_err(ApiError::crypto)?)
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
                        Err(NativeError::custom_error(
                            "You've already requested access on this Bucket!",
                        ))
                    } else {
                        BucketKey::create(id, pem, client)
                            .await
                            .map(|key| format!("\n{}", key))
                            .map_err(CliError::client_error)
                    }
                } else {
                    Err(SyncError::missing_remote_drive())
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
                    .map_err(CliError::client_error)
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
                    .map_err(CliError::client_error)
            }
            KeyCommand::Delete(ks) => {
                let (bucket_id, id) = get_key_info(client, global, &ks).await?;
                BucketKey::delete_by_id(bucket_id, id, client)
                    .await
                    .map(|id| format!("<< DELETED KEY SUCCESSFULLY >>\nid:\t{}", id))
                    .map_err(CliError::client_error)
            }
            KeyCommand::Reject(ks) => {
                let (bucket_id, id) = get_key_info(client, global, &ks).await?;
                BucketKey::reject(bucket_id, id, client)
                    .await
                    .map(|_| format!("{}", "<< REJECTED KEY SUCCESSFULLY >>".green()))
                    .map_err(CliError::client_error)
            }
        }
    }
}

async fn get_key_info(
    client: &mut Client,
    global: &GlobalConfig,
    key_specifier: &KeySpecifier,
) -> Result<(Uuid, Uuid), CliError> {
    let bucket_id = OmniBucket::from_specifier(global, client, &key_specifier.drive_specifier)
        .await
        .get_id()?;

    let all_keys = BucketKey::read_all(bucket_id, client).await?;
    let key_index = all_keys
        .iter()
        .position(|key| key.fingerprint == key_specifier.fingerprint)
        .unwrap();

    let key = all_keys[key_index].clone();

    Ok((bucket_id, key.id))
}
