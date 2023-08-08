

use anyhow::Result;
use tomb_crypt::{prelude::{WrappingPrivateKey, WrappingPublicKey}, pretty_fingerprint};
use crate::types::config::globalconfig::GlobalConfig;
use reqwest::{Url, ClientBuilder};
use super::command::ApiSubCommand;

pub async fn request(command: Request) -> Result<()> {
    let global = GlobalConfig::from_disk().await?;
    let base_url = Url::parse(&global.remote)?;
    let private_key = global.load_key().await?;
    let public_key = private_key.public_key()?;
    let public_key_pem = String::from_utf8(public_key.export().await?)?;
   
    match command {
        Request::Bucket { subcommand } => {
            match subcommand {
                super::command::BucketSubCommand::Create { name } => {
                    // Create a new interactive bucket
                    
                    println!("bucket info: {:?}", bucket_info);

                    return Ok(());
                },
                BucketRequest::List {  } => todo!(),
                BucketRequest::Get {  } => todo!(),
                BucketRequest::Delete {  } => todo!(),
            }
        },
        Request::Keys { subcommand } => {
            match subcommand {
                KeyRequest::Create {  } => todo!(),
                KeyRequest::Get {  } => todo!(),
                KeyRequest::Delete {  } => todo!(),
            }
        },
        Request::Metadata { subcommand } => todo!(),
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serial_test::serial;

    use crate::{cli::{command::{Command, BucketSubCommand, ApiSubCommand}, metadata::api}, pipelines::configure};

    #[tokio::test]
    #[serial]
    async fn bucket_create() -> Result<()> {

        let command = Request::Bucket { 
            subcommand: BucketSubCommand::Create {
                name: "test".to_string()
            } 
        };

        configure::remote("http://127.0.0.1:3001").await?;
        request(command).await?;

        Ok(())
    }
}