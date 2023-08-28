use crate::{
    cli::command::{AuthSubcommand, BanyanSubCommand},
    types::config::globalconfig::GlobalConfig,
};
use anyhow::Result;
use tomb_common::banyan_api::{
    client::{Client, Credentials},
    models::{account::Account, bucket_key::BucketKey},
    requests::auth::fake_account::create::*,
};
use tomb_crypt::prelude::{EcSignatureKey, PrivateKey, PublicKey};

pub async fn pipeline(command: BanyanSubCommand) -> Result<()> {
    println!("calling a banyan subcommand!");

    match command {
        BanyanSubCommand::Auth { subcommand } => {
            // // Start with the local client
            // let client = Client::new("http://localhost:3001")?;
            // Use global configuration of remote endpoint to create a new client.
            let mut global = GlobalConfig::from_disk().await?;

            if let Some(mut client) = global.client {
                match subcommand {
                    AuthSubcommand::CreateAccount => {
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
                    }
                    AuthSubcommand::WhoAmI => match Account::who_am_i(&mut client).await {
                        Ok(account) => {
                            println!("ACCOUNT: {:?}", account);
                        }
                        Err(err) => {
                            return Err(anyhow::Error::new(err));
                        }
                    },
                    AuthSubcommand::Usage => match Account::usage(&mut client).await {
                        Ok(val) => {
                            println!("usage: {}", val);
                        }
                        Err(err) => {
                            return Err(anyhow::Error::new(err));
                        }
                    },
                    AuthSubcommand::Limit => match Account::usage_limit(&mut client).await {
                        Ok(val) => {
                            println!("usage limit: {}", val);
                        }
                        Err(err) => {
                            return Err(anyhow::Error::new(err));
                        }
                    },
                }

                // Update the client
                global.client = Some(client);
                // Save
                global.to_disk().await?;
            } else {
                println!("asdfjas;dlkfjasd no client! configure first!");
            }
        }
        BanyanSubCommand::Key { subcommand } => {
            match subcommand {
                /// Print out a list of the keys persisted on the remote server
                crate::cli::command::KeySubcommand::List => {
                    let x: BucketKey;
                    todo!()
                }
                crate::cli::command::KeySubcommand::Approve { fingerprint } => todo!(),
                crate::cli::command::KeySubcommand::Reject { fingerprint } => todo!(),
            }
        }
    }

    Ok(())
}
