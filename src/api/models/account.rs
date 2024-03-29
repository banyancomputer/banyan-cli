use std::fmt::Display;

use crate::api::{
    client::{Client, Credentials},
    error::ApiError,
    requests::core::{
        auth::{
            fake_account::create::{CreateAccountResponse, CreateFakeAccount},
            who_am_i::read::{ReadWhoAmI, ReadWhoAmIResponse},
        },
        buckets::usage::{GetTotalUsage, GetUsageLimit},
    },
    utils::generate_api_key,
};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use tomb_crypt::prelude::EcSignatureKey;

#[derive(Debug, Deserialize, Serialize)]
/// Account Definition
pub struct Account {
    /// The unique identifier for the account
    pub id: uuid::Uuid,
}

impl Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{}\nuser_id:\t{}",
            "| ACCOUNT INFO |".yellow(),
            self.id
        ))
    }
}

impl Account {
    /// Create a new instance of this model or data structure. Attaches the associated credentials to the client.
    pub async fn create_fake(client: &mut Client) -> Result<(Self, EcSignatureKey), ApiError> {
        // Create a local key pair for signing
        let (api_key, device_api_key_pem) = generate_api_key().await;
        // Associate the key material with the backend
        let response: CreateAccountResponse = client
            .call(CreateFakeAccount { device_api_key_pem })
            .await?;

        // Associate the returned account ID with the key material and initialize the client with these credentials
        client.with_credentials(Credentials {
            user_id: response.id,
            signing_key: api_key.clone(),
        });
        // Return the account
        Ok((Self { id: response.id }, api_key))
    }

    /// Get the account associated with the current credentials in the Client
    pub async fn who_am_i(client: &mut Client) -> Result<Self, ApiError> {
        // Uhh we don't acutally need the ID for this one. There is probably a better pattern for this.
        let response: ReadWhoAmIResponse = client.call(ReadWhoAmI).await?;
        Ok(Self {
            id: response.user_id,
        })
    }

    /// Get the total usage for the account associated with the current credentials in the Client
    pub async fn usage(client: &mut Client) -> Result<u64, ApiError> {
        client
            .call(GetTotalUsage)
            .await
            .map(|response| response.size)
    }

    /// Get the usage limit for the account associated with the current credentials in the Client
    pub async fn usage_limit(client: &mut Client) -> Result<u64, ApiError> {
        client
            .call(GetUsageLimit)
            .await
            .map(|response| response.size)
    }
}

#[cfg(feature = "integration-tests")]
#[cfg(test)]
pub mod test {
    use crate::api::{client::Client, error::ApiError, models::account::Account};

    pub async fn authenticated_client() -> Client {
        let mut client = Client::new("http://127.0.0.1:3001").unwrap();
        let _ = Account::create_fake(&mut client).await.unwrap();
        client
    }

    pub async fn unauthenticated_client() -> Client {
        Client::new("http://127.0.0.1:3001").unwrap()
    }

    #[tokio::test]
    async fn who_am_i() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        println!("client: {:?}", client);
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
    async fn usage() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let usage = Account::usage(&mut client).await?;
        assert_eq!(usage, 0);
        Ok(())
    }

    #[tokio::test]
    async fn usage_limit() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let usage_limit = Account::usage_limit(&mut client).await?;
        assert_eq!(usage_limit, 50 * 1024 * 1024 * 1024);
        Ok(())
    }
}
