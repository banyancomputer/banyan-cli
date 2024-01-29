use super::RunnableCommand;
use crate::{
    api::{
        client::Credentials,
        models::account::Account,
        requests::core::auth::device_api_key::regwait::start::{
            StartRegwait, StartRegwaitResponse,
        },
    },
    native::{configuration::globalconfig::GlobalConfig, NativeError},
    prelude::api::requests::core::auth::who_am_i::read::ReadWhoAmI,
};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use bytesize::ByteSize;
use clap::Subcommand;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tomb_crypt::{
    hex_fingerprint,
    prelude::{PrivateKey, PublicKey},
};

/// Subcommand for Authentication
#[derive(Subcommand, Clone, Debug, Serialize, Deserialize)]
pub enum AccountCommand {
    /// Add Device API Key using browser session
    RegisterDevice,
    /// Log out from this device
    Logout,
    /// Register
    #[cfg(feature = "integration-tests")]
    Register,
    /// Ask the server who I am
    WhoAmI,
    /// Get info about Account usage
    Usage,
}

#[async_trait]
impl RunnableCommand<NativeError> for AccountCommand {
    async fn run_internal(self) -> Result<String, NativeError> {
        let mut global = GlobalConfig::from_disk().await?;
        let mut client = global.get_client().await?;

        // Process the command
        match self {
            AccountCommand::RegisterDevice => {
                let private_device_key = GlobalConfig::from_disk().await?.api_key().await?;
                let public_device_key = private_device_key.public_key()?;

                // If this device api key is alreaddy registered
                if client.call(ReadWhoAmI).await.is_ok() {
                    return Ok(format!("{}", "THIS DEVICE IS ALREADY REGISTERED".green()));
                }

                // Create a fingerprint from the public key
                let fingerprint =
                    hex_fingerprint(public_device_key.fingerprint().await?.as_slice());

                // Create a new request object with the nonce
                let start_regwait = StartRegwait {
                    fingerprint: fingerprint.clone(),
                };
                // Create a clone of the client to move into the handle
                let mut client_1 = client.clone();
                // Create a join handle for later use, starting the call immediately
                let join_handle: JoinHandle<Result<StartRegwaitResponse, String>> =
                    tokio::spawn(async move {
                        // Build the request
                        client_1
                            .call(start_regwait)
                            .await
                            .map_err(|err| err.to_string())
                    });

                // URL encoded DER bytes
                let spki_b64 =
                    general_purpose::STANDARD.encode(public_device_key.export_bytes().await?);
                let spki_b64_url_safe =
                    url::form_urlencoded::byte_serialize(spki_b64.as_bytes()).collect::<String>();
                // Construct the proper URL to open
                let url = global
                    .get_endpoint()
                    .join(&format!("register-device/{}", spki_b64_url_safe))
                    .unwrap();

                // Offer the link directly if it fails to open
                let _ = open::that(url.as_str());
                info!(
                    "open this link in your browser:\n{}",
                    url.as_str().bright_blue()
                );

                // Now await the completion of the original request
                let start_response = join_handle
                    .await
                    .map_err(|err| NativeError::custom_error(&err.to_string()))?
                    .map_err(|msg| NativeError::custom_error(&msg))?;

                // Update the client's credentials
                client.with_credentials(Credentials {
                    user_id: start_response.user_id,
                    signing_key: private_device_key,
                });

                global.save_client(client).await?;

                // Respond
                Ok(format!(
                    "{}\nuser_id:\t\t{}\ndevice_key_fingerprint:\t{}",
                    "<< DEVICE KEY SUCCESSFULLY ADDED TO ACCOUNT >>".green(),
                    start_response.user_id,
                    fingerprint
                ))
            }
            AccountCommand::Logout => {
                client.logout();
                global.save_client(client).await?;
                Ok(format!(
                    "{}",
                    "<< SUCCESSFULLY LOGGED OUT OF REMOTE ACCESS >>".green()
                ))
            }
            #[cfg(feature = "integration-tests")]
            AccountCommand::Register => {
                // Additional imports
                use crate::api::requests::core::auth::fake_account::create::{
                    CreateAccountResponse, CreateFakeAccount,
                };
                use tomb_crypt::prelude::EcSignatureKey;

                // Create local keys
                let api_key = EcSignatureKey::generate().await?;
                let public_api_key = api_key.public_key()?;
                let public_api_key_pem = String::from_utf8(public_api_key.export().await?)?;
                // Associate the key material with the backend
                let response: CreateAccountResponse = client
                    .call(CreateFakeAccount {
                        device_api_key_pem: public_api_key_pem,
                    })
                    .await?;
                client.with_credentials(Credentials {
                    user_id: response.id,
                    signing_key: api_key.clone(),
                });

                Ok(format!(
                    "{}\nuser_id:\t{}",
                    "<< CREATED NEW ACCOUNT >>".green(),
                    response.id
                ))
            }
            AccountCommand::WhoAmI => Account::who_am_i(&mut client)
                .await
                .map(|v| v.to_string())
                .map_err(NativeError::api),
            AccountCommand::Usage => {
                let mut output = format!("{}", "| ACCOUNT USAGE INFO |".yellow());

                let usage_current_result = Account::usage(&mut client).await;
                let usage_limit_result = Account::usage_limit(&mut client).await;

                if usage_current_result.is_err() && usage_limit_result.is_err() {
                    return Err(NativeError::custom_error(
                        "Unable to obtain usage stats. Check your authentication!",
                    ));
                }

                if let Ok(usage_current) = usage_current_result {
                    output = format!("{}\nusage_current:\t{}", output, ByteSize(usage_current));
                }
                if let Ok(usage_limit) = usage_limit_result {
                    output = format!("{}\nusage_limit:\t{}", output, ByteSize(usage_limit));
                }

                Ok(output)
            }
        }
    }
}
