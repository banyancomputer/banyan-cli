use super::RunnableCommand;
use crate::{
    api::{
        client::{Client, Credentials},
        error::ClientError,
        models::account::Account,
        requests::core::auth::device_api_key::regwait::start::{
            StartRegwait, StartRegwaitResponse,
        },
    },
    native::configuration::globalconfig::GlobalConfig,
};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use bytesize::ByteSize;
use clap::Subcommand;
use colored::Colorize;
use tokio::task::JoinHandle;
use tomb_crypt::{
    hex_fingerprint,
    prelude::{PrivateKey, PublicKey},
};

/// Subcommand for Authentication
#[derive(Subcommand, Clone, Debug)]
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

#[async_trait(?Send)]
impl RunnableCommand<ClientError> for AccountCommand {
    async fn run_internal(
        self,
        _: &mut GlobalConfig,
        client: &mut Client,
    ) -> Result<String, ClientError> {
        // Process the command
        match self {
            AccountCommand::RegisterDevice => {
                // let device_key = EcEncryptionKey::generate().await?;
                let private_device_key = GlobalConfig::from_disk().await?.api_key().await?;

                // Create a public key from the
                let public_device_key = private_device_key
                    .public_key()
                    .map_err(ClientError::crypto_error)?;

                // Create a fingerprint from the public key
                let fingerprint =
                    hex_fingerprint(public_device_key.fingerprint().await?.as_slice());
                // URL encoded DER bytes
                let der_url = general_purpose::URL_SAFE_NO_PAD
                    .encode(public_device_key.export_bytes().await?);
                // Create a new request object with the nonce
                let start_regwait = StartRegwait {
                    fingerprint: fingerprint.clone(),
                };
                // Create a clone of the client to move into the handle
                let mut client_1 = client.clone();
                // Create a join handle for later use, starting the call immediately
                let join_handle: JoinHandle<Result<StartRegwaitResponse, ClientError>> =
                    tokio::spawn(async move {
                        // Build the request
                        client_1.call(start_regwait).await
                    });

                // Open this url
                open::that(format!(
                    "{}/completedevicekey?spki={}",
                    GlobalConfig::from_disk().await?.endpoints.frontend,
                    der_url
                ))
                .expect("failed to open browser");

                // Now await the completion of the original request
                let start_response = join_handle
                    .await
                    .map_err(anyhow::Error::new)
                    .map(|v| v.map_err(anyhow::Error::new))??;

                // Update the client's credentials
                client.with_credentials(Credentials {
                    user_id: start_response.user_id,
                    signing_key: private_device_key,
                });

                // Respond
                Ok(format!(
                    "{}\nuser_id:\t{}\ndevice_key_fingerprint:\t{}",
                    "<< DEVICE KEY SUCCESSFULLY ADDED TO ACCOUNT >>".green(),
                    start_response.user_id,
                    fingerprint
                ))
            }
            AccountCommand::Logout => {
                client.logout();
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
                let public_api_key_pem = String::from_utf8(public_api_key.export().await?)
                    .map_err(|_| ClientError::custom_error("utf8 PEM"))?;
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
            AccountCommand::WhoAmI => Account::who_am_i(client).await.map(|v| v.to_string()),
            AccountCommand::Usage => {
                let mut output = format!("{}", "| ACCOUNT USAGE INFO |".yellow());

                let usage_current_result = Account::usage(client).await;
                let usage_limit_result = Account::usage_limit(client).await;

                if usage_current_result.is_err() && usage_limit_result.is_err() {
                    return Err(ClientError::custom_error(
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
