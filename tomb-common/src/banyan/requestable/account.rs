use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::banyan::client::Client;
use crate::banyan::requestable::{Requestable, RequestableError};
use crate::banyan::requests::auth::who_am_i::read::*;

#[cfg(test)]
use {
    tomb_crypt::prelude::*,
    crate::banyan::requests::auth::fake_account::create::*
};


#[cfg(test)]
pub async fn generate_api_key() ->
    (
        EcSignatureKey,
        String
    ) 
 {
    let api_key = EcSignatureKey::generate().await.unwrap();
    let public_api_key = api_key.public_key().unwrap();
    let public_api_key_pem = String::from_utf8(public_api_key.export().await.unwrap()).unwrap();
    (api_key, public_api_key_pem)
}

#[derive(Debug, Deserialize, Serialize)]
/// Account Definition
pub struct Account {
    pub id: Option<uuid::Uuid>,
}

#[async_trait(?Send)]
impl Requestable for Account {
    type ErrorType = RequestableError; 

    fn id(&self) -> Result<uuid::Uuid, Self::ErrorType> {
        match self.id {
            Some(id) => Ok(id),
            None => Err(RequestableError::missing_id())
        }
    }

    /// Create a new instance of this model or data structure. Attaches the associated credentials to the client.
    async fn create(self: Self, _client: &mut Client) -> Result<Self, Self::ErrorType> {
        #[cfg(test)]
        {
            use crate::banyan::credentials::Credentials;
            // Create a local key pair for signing
            let (api_key, device_api_key_pem) = generate_api_key().await;
            // Associate the key material with the backend
            let response: CreateAccountResponse = _client.call(CreateAccount {
                device_api_key_pem
            }).await.map_err(|_| RequestableError::client_error())?;

            // Associate the returned account ID with the key material and initialize the client with these credentials
            _client.with_credentials(Credentials {
                account_id: response.id,
                signing_key: api_key.clone(),
            });
            // Return the account
            Ok(Self {
                id: Some(response.id),
            })
        }
        #[cfg(not(test))]
        {
            // Fail if called in production
            Err(RequestableError::unsupported_request())
        }
    }

    /// Fail if called
    async fn read_all(_client: &mut Client) -> Result<Vec<Self>, Self::ErrorType> {
        Err(RequestableError::unsupported_request())
    }

    /// Get the account associated with the current credentials. You do not need to pass an ID for this request.
    async fn read(client: &mut Client, _id: &str) -> Result<Self, Self::ErrorType> {
        // Uhh we don't acutally need the ID for this one. There is probably a better pattern for this.
        let response: ReadWhoAmIResponse = client.call(ReadWhoAmI).await.map_err(|_| RequestableError::client_error())?;
        Ok(Self {
            id: Some(response.account_id),
        })
    }

    /// Fail if called
    async fn update(self: Self, _client: &mut Client, _id: &str) -> Result<Self, Self::ErrorType> {
        Err(RequestableError::unsupported_request())
    }

    /// Fail if called
    async fn delete(_client: &mut Client, _id: &str) -> Result<Self, Self::ErrorType> {
        Err(RequestableError::unsupported_request())
    }
}

// TODO: wasm tests

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::banyan::client::Client;

    pub async fn authenticated_client() -> Client {
        let mut client = Client::new("http://localhost:3001").unwrap();
        let create = Account::create(Account {
            id: None,
        }, &mut client).await.unwrap();
        assert!(create.id.is_some());
        client
    }

    #[tokio::test]
    async fn create_read() -> Result<(), RequestableError> {
        let mut client = authenticated_client().await; 
        let create = Account::create(Account {
            id: None,
        }, &mut client).await?;
        let read= Account::read(&mut client, "").await?;
        assert!(read.id.is_some());
        assert_eq!(create.id, read.id);
        Ok(())
    }
}