use crate::{cli::command::AuthSubCommand, types::config::globalconfig::GlobalConfig};
use anyhow::Result;
use tomb_common::banyan_api::{client::Credentials, error::ClientError, models::account::Account};

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

            // Register the device and
            let account =
                Account::register_device(client.clone(), private_device_key.clone()).await?;

            // Update the client's credentials
            client.with_credentials(Credentials {
                account_id: account.id,
                signing_key: private_device_key,
            });

            // Format
            Ok(format!(""))
        }
        #[cfg(feature = "fake")]
        AuthSubCommand::Register => {
            // Additional imports
            use tomb_common::banyan_api::requests::core::auth::fake_account::create::{
                CreateAccountResponse, CreateFakeAccount,
            };
            use tomb_crypt::prelude::{EcSignatureKey, PrivateKey, PublicKey};
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
