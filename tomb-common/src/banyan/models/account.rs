use serde::{Deserialize, Serialize};

#[cfg(feature = "banyan-api")]
use crate::banyan::{api::auth::who_am_i::read::*, client::Client, error::ClientError};

#[cfg(feature = "banyan-api")]
#[cfg(test)]
use {crate::banyan::api::auth::fake_account::create::*, tomb_crypt::prelude::*};

#[cfg(feature = "banyan-api")]
#[cfg(test)]
pub async fn generate_api_key() -> (EcSignatureKey, String) {
    let api_key = EcSignatureKey::generate().await.unwrap();
    let public_api_key = api_key.public_key().unwrap();
    let public_api_key_pem = String::from_utf8(public_api_key.export().await.unwrap()).unwrap();
    (api_key, public_api_key_pem)
}

#[cfg(feature = "banyan-api")]
#[cfg(test)]
pub async fn generate_bucket_key() -> (EcEncryptionKey, String) {
    let bucket_key = EcEncryptionKey::generate().await.unwrap();
    let public_bucket_key = bucket_key.public_key().unwrap();
    let public_bucket_key_pem =
        String::from_utf8(public_bucket_key.export().await.unwrap()).unwrap();
    (bucket_key, public_bucket_key_pem)
}

#[derive(Debug, Deserialize, Serialize)]
/// Account Definition
pub struct Account {
    /// The unique identifier for the account
    pub id: uuid::Uuid,
}

#[cfg(feature = "banyan-api")]
impl Account {
    #[cfg(test)]
    /// Create a new instance of this model or data structure. Attaches the associated credentials to the client.
    pub async fn create_fake(_client: &mut Client) -> Result<Self, ClientError> {
        use crate::banyan::credentials::Credentials;
        // Create a local key pair for signing
        let (api_key, device_api_key_pem) = generate_api_key().await;
        // Associate the key material with the backend
        let response: CreateAccountResponse =
            _client.call(CreateAccount { device_api_key_pem }).await?;

        // Associate the returned account ID with the key material and initialize the client with these credentials
        _client.with_credentials(Credentials {
            account_id: response.id,
            signing_key: api_key.clone(),
        });
        // Return the account
        Ok(Self { id: response.id })
    }

    /// Get the account associated with the current credentials in the Client
    pub async fn who_am_i(client: &mut Client) -> Result<Self, ClientError> {
        // Uhh we don't acutally need the ID for this one. There is probably a better pattern for this.
        let response: ReadWhoAmIResponse = client.call(ReadWhoAmI).await?;
        Ok(Self {
            id: response.account_id,
        })
    }
}

// TODO: wasm tests

#[cfg(feature = "banyan-api")]
#[cfg(test)]
pub mod test {
    use super::*;
    use crate::banyan::client::Client;

    pub async fn authenticated_client() -> Client {
        let mut client = Client::new("http://localhost:3001").unwrap();
        let _ = Account::create_fake(&mut client).await.unwrap();
        client
    }

    #[tokio::test]
    #[ignore]
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
    #[ignore]
    async fn who_am_i_unauthenticated() {
        let mut client = Client::new("http://localhost:3001").unwrap();
        let _ = Account::who_am_i(&mut client).await.unwrap();
    }
}
