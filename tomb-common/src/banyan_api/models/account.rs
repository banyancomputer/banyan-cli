use crate::banyan_api::{
    client::{Client, Credentials},
    error::ClientError,
    requests::core::{
        auth::{
            device_api_key::regwait::start::{StartRegwait, StartRegwaitResponse},
            fake_account::create::*,
            who_am_i::read::*,
        },
        buckets::usage::{GetTotalUsage, GetUsageLimit},
    },
    utils::generate_api_key,
};
use anyhow::anyhow;
use futures::executor::block_on;
use futures_core::Future;
use futures_util::FutureExt;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    str::FromStr,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tokio::{runtime::Handle, time::timeout};
use tomb_crypt::{
    prelude::{EcSignatureKey, PrivateKey, PublicKey},
    pretty_fingerprint,
};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
/// Account Definition
pub struct Account {
    /// The unique identifier for the account
    pub id: uuid::Uuid,
}

impl Account {
    /// Create a new instance of this model or data structure. Attaches the associated credentials to the client.
    pub async fn create_fake(client: &mut Client) -> Result<(Self, EcSignatureKey), ClientError> {
        // Create a local key pair for signing
        let (api_key, device_api_key_pem) = generate_api_key().await;
        // Associate the key material with the backend
        let response: CreateAccountResponse = client
            .call_core(CreateFakeAccount { device_api_key_pem })
            .await?;

        // Associate the returned account ID with the key material and initialize the client with these credentials
        client.with_credentials(Credentials {
            account_id: response.id,
            signing_key: api_key.clone(),
        });
        // Return the account
        Ok((Self { id: response.id }, api_key))
    }

    /// Log in to an existing account
    pub async fn register_device(
        client: &mut Client,
        private_device_key: EcSignatureKey,
    ) -> Result<Self, ClientError> {
        let public_device_key = private_device_key
            .public_key()
            .expect("failed to create public key");
        // let public_device_key_fingerprint = pretty_fingerprint(&public_device_key.fingerprint().await.expect("unable to generate fingerprint"));
        // Public device key in PEM format
        let public_device_key =
            String::from_utf8(public_device_key.export().await.expect("cant export key"))
                .expect("cant convert key bytes to string");
        // Strip the public key of its new lines
        let mut stripped_public_key = public_device_key.replace('\n', "");
        // Strip the public key of its prefix and suffix
        stripped_public_key = stripped_public_key
            .strip_prefix("-----BEGIN PUBLIC KEY-----")
            .unwrap()
            .strip_suffix("-----END PUBLIC KEY-----")
            .unwrap()
            .to_string();

        // Represent the weird b64 characters with ones that are url-valid
        let encoded_public_key = stripped_public_key
            .replace('+', "-")
            .replace('/', "_")
            .replace('=', ".")
            .to_string();
        println!("the stripped public key:\n ~{}~", stripped_public_key);
        println!("the encoded public key:\n ~{}~", encoded_public_key);

        // Start a background task

        // Create a new object for the registration wait task
        let start_regwait = StartRegwait::new();
        // Create a base64 url encoded version of the associated nonce
        let b64_nonce = base64_url::encode(&start_regwait.nonce.to_string());
        let mut client_1 = client.clone();
        let join_handle = tokio::task::spawn_blocking(move || {
            println!("calling core...");
            let future = client_1.call_core(start_regwait);
            Handle::current().block_on(future)
        });
        println!("the join handle has been created!");

        // Base url for the frontend
        let base_url = "http://127.0.0.1:3000";
        // Should be this in prod TODO
        // https://alpha.data.banyan.computer/

        // Open this url with firefox
        open::with(
            format!(
                "{}/api/auth/device/register?spki={}&nonce={}",
                base_url, encoded_public_key, b64_nonce
            ),
            "firefox",
        )
        .expect("failed to open browser");

        println!("url opened!");

        // Now that the url has been opened, await the join handle
        match join_handle.await {
            Ok(Ok(response)) => {
                // Update credentials
                client.with_credentials(Credentials {
                    account_id: response.account_id,
                    signing_key: private_device_key,
                });
                // Ok
                Ok(Self {
                    id: response.account_id,
                })
            }
            Ok(Err(err)) => {
                println!("client error!: {}", err);
                Err(err)
            }
            Err(err) => {
                println!("join error!: {}", err);
                todo!()
            }
        }
    }

    /// Get the account associated with the current credentials in the Client
    pub async fn who_am_i(client: &mut Client) -> Result<Self, ClientError> {
        // Uhh we don't acutally need the ID for this one. There is probably a better pattern for this.
        let response: ReadWhoAmIResponse = client.call_core(ReadWhoAmI).await?;
        Ok(Self {
            id: response.account_id,
        })
    }

    /// Get the total usage for the account associated with the current credentials in the Client
    pub async fn usage(client: &mut Client) -> Result<u64, ClientError> {
        let response = client.call_core(GetTotalUsage).await?;
        Ok(response.size as u64)
    }

    /// Get the usage limit for the account associated with the current credentials in the Client
    pub async fn usage_limit(client: &mut Client) -> Result<u64, ClientError> {
        let response = client.call_core(GetUsageLimit).await?;
        Ok(response.size as u64)
    }
}

// TODO: wasm tests

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::banyan_api::client::Client;

    pub async fn authenticated_client() -> Client {
        let mut client = Client::new("http://localhost:3001", "http://localhost:3002").unwrap();
        let _ = Account::create_fake(&mut client).await.unwrap();
        client
    }

    pub async fn unauthenticated_client() -> Client {
        Client::new("http://localhost:3001", "http://localhost:3002").unwrap()
    }

    #[tokio::test]
    async fn who_am_i() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let subject = client.subject().unwrap();
        let read = Account::who_am_i(&mut client).await?;
        let subject_uuid = uuid::Uuid::parse_str(&subject).unwrap();
        assert_eq!(subject_uuid, read.id);
        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn who_am_i_unauthenticated() {
        let mut client = unauthenticated_client().await;
        let _ = Account::who_am_i(&mut client).await.unwrap();
    }

    #[tokio::test]
    async fn usage() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let usage = Account::usage(&mut client).await?;
        assert_eq!(usage, 0);
        Ok(())
    }

    #[tokio::test]
    async fn usage_limit() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let usage_limit = Account::usage_limit(&mut client).await?;
        // 5 TiB
        assert_eq!(usage_limit, 5 * 1024 * 1024 * 1024 * 1024);
        Ok(())
    }

    #[tokio::test]
    async fn register_device() -> Result<(), ClientError> {
        let mut client = unauthenticated_client().await;
        let private_device_key = EcSignatureKey::generate().await.unwrap();
        // let public_key = private_key.public_key().unwrap();
        // let fingerprint = pretty_fingerprint(&public_key.fingerprint().await.unwrap());

        let account = Account::register_device(&mut client, private_device_key).await?;
        println!("account: {:?}", account);

        Ok(())
    }

    // #[tokio::test]
    // async fn regwait_start() -> Result<(), ClientError> {
    //     let mut client = unauthenticated_client().await;
    //     let private_key = EcSignatureKey::generate().await.unwrap();
    //     let public_key = private_key.public_key().unwrap();
    //     let fingerprint = pretty_fingerprint(&public_key.fingerprint().await.unwrap());

    //     // Call the start_regwait funtion
    //     let response = client.call_core(StartRegwait { fingerprint }).await?;

    //     println!("response: {:?}", response);

    //     Ok(())
    // }
}
