use crate::{
    cli::command::{AuthSubcommand, BanyanSubCommand},
    types::config::globalconfig::GlobalConfig,
};
use anyhow::{Result, anyhow};
use tomb_common::banyan_api::{
    client::{Client, Credentials},
    models::{account::Account, bucket_key::BucketKey, bucket::{Bucket, StorageClass, BucketType}},
    requests::{auth::fake_account::create::*, buckets::read::ReadAllBuckets}, error::ClientError,
};
use tomb_crypt::prelude::{EcSignatureKey, PrivateKey, PublicKey, EcEncryptionKey};

pub async fn pipeline(command: BanyanSubCommand) -> Result<String> {
    println!("calling a banyan subcommand!");
    let mut global = GlobalConfig::from_disk().await?;
    // If there is a Client configured
    if let Some(mut client) = global.client {
        // Process the command
        let result: Result<String, ClientError> = match command {
            // If it is an auth command
            BanyanSubCommand::Auth { subcommand } => {
                // // Start with the local client
                // let client = Client::new("http://localhost:3001")?;
                // Use global configuration of remote endpoint to create a new client.
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

                        Ok(format!("Created account with id: {}", response.id))
                    }
                    AuthSubcommand::WhoAmI => Account::who_am_i(&mut client).await.map(|v| format!("account: {}", v.id)),
                    AuthSubcommand::Usage => Account::usage(&mut client).await.map(|v| format!("usage: {}", v)),
                    AuthSubcommand::Limit => Account::usage_limit(&mut client).await.map(|v| format!("usage limit: {}", v)),
                }
            }
            BanyanSubCommand::Bucket { subcommand } => {
                match subcommand {
                    crate::cli::command::BucketSubcommand::Create { name } => {
                        let private_key = EcEncryptionKey::generate().await?;
                        let public_key = private_key.public_key()?;
                        let pem = String::from_utf8(public_key.export().await?)?;
                        Bucket::create(name, pem, BucketType::Interactive, StorageClass::Hot, &mut client).await.map(|(b, k)| format!("new bucket: {:?}\nnew bucket key: {:?}", b, k))
                    },
                    crate::cli::command::BucketSubcommand::Delete => todo!(),
                    crate::cli::command::BucketSubcommand::Usage => todo!(),
                    crate::cli::command::BucketSubcommand::Read { id } => {
                        if let Some(id) = id {
                            Bucket::read(&mut client, id).await.map(|v| format!("bucket info: {:?}", v))
                        } else {
                            Bucket::read_all(&mut client).await.map(|v| format!("buckets: {:?}", v))
                        }
                    },
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
        };

        // Update the client
        global.client = Some(client);
        // Save
        global.to_disk().await?;

        result.map_err(anyhow::Error::new)

        // Err(anyhow!("dfasd"))
    }
    else {
        Err(anyhow!("dfasd"))
    }

    // Ok(())
}
