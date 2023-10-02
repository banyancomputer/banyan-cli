use crate::{cli::command::AuthSubCommand, types::config::globalconfig::GlobalConfig};
use anyhow::Result;
use tokio::task::JoinHandle;
use tomb_common::banyan_api::{
    client::Credentials, error::ClientError, models::account::Account,
    requests::core::auth::device_api_key::regwait::*,
};
use tomb_crypt::prelude::{PrivateKey, PublicKey};
use uuid::Uuid;

/// Handle Auth management both locally and remotely based on CLI input
pub async fn pipeline(command: AuthSubCommand) -> Result<String> {
    // Grab global config
    let mut global = GlobalConfig::from_disk().await?;
    // Obtain the Client
    let mut client = global.get_client().await?;

    // Process the command
    let result: Result<String, ClientError> = match command {
        AuthSubCommand::RegisterDevice => {
            // let device_key = EcEncryptionKey::generate().await?;
            let private_device_key = GlobalConfig::from_disk().await?.api_key().await?;

            // Create a public key from the
            let public_device_key = private_device_key
                .public_key()
                .map_err(ClientError::crypto_error)?;
            // Bytes of the public device key
            let public_device_key_bytes = public_device_key
                .export()
                .await
                .map_err(ClientError::crypto_error)?;

            // Public device key in PEM format
            let public_device_key = String::from_utf8(public_device_key_bytes)
                .expect("cant convert key bytes to string");
            // Strip the public key of its new lines
            let mut stripped_public_key = public_device_key.replace('\n', "");
            // Strip the public key of its prefix and suffix
            stripped_public_key = stripped_public_key
                .strip_prefix("-----BEGIN PUBLIC KEY-----")
                .expect("unable to strip PEM prefix")
                .strip_suffix("-----END PUBLIC KEY-----")
                .expect("unable to strip PEM suffix")
                .to_string();

            // Represent the weird b64 characters with ones that are url-valid
            let encoded_public_key = stripped_public_key
                .replace('+', "-")
                .replace('/', "_")
                .replace('=', ".")
                .to_string();

            // Create a new nonce to identify this registration
            let nonce = Uuid::new_v4();
            // Create a new request object with the nonce
            let start_regwait = Regwait { nonce };
            // Create a base64 url encoded version of the nonce
            let b64_nonce = base64_url::encode(&nonce);
            // Create a clone of the client to move into the handle
            let mut client_1 = client.clone();
            // Create a join handle for later use, starting the call immediately
            let join_handle: JoinHandle<Result<RegwaitResponse, ClientError>> =
                tokio::task::spawn(async move {
                    // Build the request
                    client_1.call(start_regwait).await
                });

            // Should be this in prod TODO
            // https://alpha.data.banyan.computer/

            // Open this url with firefox
            open::with(
                format!(
                    "{}/api/auth/device/register?spki={}&nonce={}",
                    global
                        .clone()
                        .remote_frontend
                        .expect("no frontend url configured"),
                    encoded_public_key,
                    b64_nonce
                ),
                "firefox",
            )
            .expect("failed to open browser");

            // Now that the url has been opened, await the join handle
            match join_handle.await {
                Ok(Ok(response)) => {
                    // Update the client's credentials
                    client.with_credentials(Credentials {
                        account_id: response.account_id,
                        signing_key: private_device_key,
                    });

                    // Ok
                    Ok("new device registered".to_string())
                }
                Ok(Err(err)) => Err(err),
                Err(err) => Err(ClientError::custom_error(&format!(
                    "joining error: {}",
                    err
                ))),
            }
        }
        #[cfg(feature = "fake")]
        AuthSubCommand::Register => {
            // Additional imports
            use tomb_common::banyan_api::requests::core::auth::fake_account::create::{
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
                account_id: response.id,
                signing_key: api_key.clone(),
            });

            Ok(format!("created account with id: {}", response.id))
        }
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
    global.to_disk()?;

    // Return
    result.map_err(anyhow::Error::new)
}
