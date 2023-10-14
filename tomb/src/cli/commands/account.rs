use crate::types::config::globalconfig::GlobalConfig;
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use clap::Subcommand;
use tokio::task::JoinHandle;
use tomb_common::banyan_api::{
    client::{Client, Credentials},
    error::ClientError,
    models::account::Account,
    requests::core::auth::device_api_key::regwait::start::{StartRegwait, StartRegwaitResponse},
};
use tomb_crypt::{
    prelude::{EcSignatureKey, PrivateKey, PublicKey},
    pretty_fingerprint,
};

use super::RunnableCommand;

/// Subcommand for Authentication
#[derive(Subcommand, Clone, Debug)]
pub enum AccountCommand {
    /// Add Device API Key
    RegisterDevice,
    /// Log out from this device
    Logout,
    // /// Register
    // #[cfg(feature = "fake")]
    // Register,
    /// Ask the server who I am
    WhoAmI,
    /// Ask the server my usage
    Usage,
    /// Ask the server my usage limit
    Limit,
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
                // Register this device key
                let credentials = register_device(client, private_device_key).await?;
                // Update client authentication
                client.with_credentials(credentials);
                Ok("account registered".to_string())
            }
            AccountCommand::Logout => {
                client.logout();
                Ok("successfully logged out".to_string())
            }
            // #[cfg(feature = "fake")]
            // AccountCommand::Register => {
            //     // Additional imports
            //     use tomb_common::banyan_api::requests::core::auth::fake_account::create::{
            //         CreateAccountResponse, CreateFakeAccount,
            //     };
            //     // Create local keys
            //     let api_key = EcSignatureKey::generate().await.map_err(ClientError::crypto_error).map_err(TombError::client_error);
            //     let public_api_key = api_key.public_key()?;
            //     let public_api_key_pem = String::from_utf8(public_api_key.export().await?)?;
            //     // Associate the key material with the backend
            //     let response: CreateAccountResponse = client
            //         .call(CreateFakeAccount {
            //             device_api_key_pem: public_api_key_pem,
            //         })
            //         .await?;
            //     client.with_credentials(Credentials {
            //         account_id: response.id,
            //         signing_key: api_key.clone(),
            //     });

            //     Ok(format!("created account with id: {}", response.id))
            // }
            AccountCommand::WhoAmI => Account::who_am_i(client)
                .await
                .map(|v| format!("account: {}", v.id)),
            AccountCommand::Usage => Account::usage(client)
                .await
                .map(|v| format!("usage: {}", v)),
            AccountCommand::Limit => Account::usage_limit(client)
                .await
                .map(|v| format!("usage limit: {}", v)),
        }
    }
}

async fn register_device(
    client: &Client,
    private_device_key: EcSignatureKey,
) -> anyhow::Result<Credentials> {
    // Create a public key from the
    let public_device_key = private_device_key
        .public_key()
        .map_err(ClientError::crypto_error)?;

    // Create a fingerprint from the public key
    let fingerprint = pretty_fingerprint(public_device_key.fingerprint().await?.as_slice());
    // URL encoded DER bytes
    let der_url = general_purpose::URL_SAFE_NO_PAD.encode(public_device_key.export_bytes().await?);
    // Create a new request object with the nonce
    let start_regwait = StartRegwait { fingerprint };
    // Create a clone of the client to move into the handle
    let mut client_1 = client.clone();
    // Create a join handle for later use, starting the call immediately
    let join_handle: JoinHandle<Result<StartRegwaitResponse, ClientError>> =
        tokio::spawn(async move {
            // Build the request
            client_1.call(start_regwait).await
        });

    // Open this url with firefox
    open::with(
        format!(
            "{}/completedevicekey?spki={}",
            GlobalConfig::from_disk().await?.endpoints.frontend,
            der_url
        ),
        "firefox",
    )
    .expect("failed to open browser");

    //
    let start_response = join_handle
        .await
        .map_err(anyhow::Error::new)
        .map(|v| v.map_err(anyhow::Error::new))??;

    // Update the client's credentials
    Ok(Credentials {
        account_id: start_response.account_id,
        signing_key: private_device_key,
    })
}

#[cfg(feature = "fake")]
#[cfg(test)]
mod test {
    use anyhow::Result;
    use tomb_common::banyan_api::client::Client;
    use tomb_crypt::prelude::{EcSignatureKey, PrivateKey};

    #[tokio::test]
    async fn register_new_device() -> Result<()> {
        let client = Client::new("http://localhost:3001", "http://localhost:3002").unwrap();
        let private_device_key = EcSignatureKey::generate().await.unwrap();
        let credentials = super::register_device(&client, private_device_key).await?;
        println!("credentials: {:?}", credentials);
        Ok(())
    }
}
