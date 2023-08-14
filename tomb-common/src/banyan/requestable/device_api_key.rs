use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::banyan::client::Client;
use crate::banyan::requestable::{Requestable, RequestableError};
use crate::banyan::requests::auth::device_api_key::{
    create::*,
    read::*,
    delete::*,
};

#[derive(Debug, Deserialize, Serialize)]
/// DeviceApiKey Definition. Note the options for the fields.
/// This does not apply to the Db models. It just allows working with structs as partials.
pub struct DeviceApiKey {
    pub id: Option<uuid::Uuid>,
    pub account_id: Option<uuid::Uuid>,
    pub fingerprint: Option<String>,
    pub pem: Option<String>,
}

impl DeviceApiKey {
    pub fn new(pem: String) -> Self {
        Self {
            id: None,
            account_id: None,
            fingerprint: None,
            pem: Some(pem),
        }
    }

    pub fn account_id(&self) -> Result<uuid::Uuid, RequestableError> {
        self.account_id.clone().ok_or(RequestableError::missing_field("account_id".into()))
    }

    pub fn pem(&self) -> Result<String, RequestableError> {
        self.pem.clone().ok_or(RequestableError::missing_field("pem".into()))
    }

    pub fn fingerprint(&self) -> Result<String, RequestableError> {
        self.fingerprint.clone().ok_or(RequestableError::missing_field("fingerprint".into()))
    }
}

#[async_trait(?Send)]
impl Requestable for DeviceApiKey  {
    type ErrorType = RequestableError; 

    fn id(&self) -> Result<uuid::Uuid, Self::ErrorType> {
        match self.id {
            Some(id) => Ok(id),
            None => Err(RequestableError::missing_id())
        }
    }

    /// Create a new instance of this model or data structure. Attaches the associated credentials to the client.
    async fn create(self: Self, client: &mut Client) -> Result<Self, Self::ErrorType> {
        let response: CreateDeviceApiKeyResponse = client.call(CreateDeviceApiKey {
            pem: self.pem()?
        }).await.map_err(|_| RequestableError::client_error())?;
        Ok(Self {
            id: Some(response.id),
            account_id: Some(response.account_id),
            fingerprint: Some(response.fingerprint),
            pem: self.pem
        })
    }

    /// Read all instances of this model or data structure.
    async fn read_all(client: &mut Client) -> Result<Vec<Self>, Self::ErrorType> {
        let response: ReadAllDeviceApiKeysResponse = client.call(ReadAllDeviceApiKeys).await.map_err(|_| RequestableError::client_error())?;
        // Map the response to the model
        let mut device_api_keys = Vec::new();
        for device_api_key in response.0 {
            device_api_keys.push(Self {
                id: Some(device_api_key.id),
                account_id: Some(device_api_key.account_id),
                fingerprint: Some(device_api_key.fingerprint),
                pem: Some(device_api_key.pem),
            });
        }
        Ok(device_api_keys)
    }

    /// Get the account associated with the current credentials. You do not need to pass an ID for this request.
    async fn read(client: &mut Client, id: &str) -> Result<Self, Self::ErrorType> {
        let response: ReadDeviceApiKeyResponse = client.call(ReadDeviceApiKey {
            id: uuid::Uuid::parse_str(id).unwrap()
        }).await.map_err(|_| RequestableError::client_error())?;
        Ok(Self {
            id: Some(response.id),
            account_id: Some(response.account_id),
            fingerprint: Some(response.fingerprint),
            pem: Some(response.pem),
        })
    }

    /// Fail if called
    async fn update(self: Self, _client: &mut Client, _id: &str) -> Result<Self, Self::ErrorType> {
        Err(RequestableError::unsupported_request())
    }

    /// Fail if called
    async fn delete(_client: &mut Client, _id: &str) -> Result<Self, Self::ErrorType> {
        let response: DeleteDeviceApiKeyResponse = _client.call(DeleteDeviceApiKey {
            id: uuid::Uuid::parse_str(_id).unwrap()
        }).await.map_err(|_| RequestableError::client_error())?;
        Ok(Self {
            id: Some(response.id),
            account_id: Some(response.account_id),
            fingerprint: Some(response.fingerprint),
            pem: None
        }) 
    }
}

// TODO: wasm tests

#[cfg(test)]
mod test {
    use super::*;
    use crate::banyan::requestable::account::generate_api_key;
    use crate::banyan::requestable::account::test::authenticated_client;

    #[tokio::test]
    async fn create() -> Result<(), RequestableError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        println!("pem: {:?}", pem);
        let device_api_key = DeviceApiKey::new(pem);
        let create = device_api_key.create(&mut client).await?;
        assert!(create.id.is_some());
        assert!(create.account_id.is_some());
        assert!(create.fingerprint.is_some());
        assert!(create.pem.is_some());
        Ok(())
    }

    #[tokio::test]
    async fn create_read() -> Result<(), RequestableError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        let device_api_key = DeviceApiKey::new(pem);
        let create = device_api_key.create(&mut client).await?;
        let read = DeviceApiKey::read(&mut client, &create.id().unwrap().to_string()).await?;
        assert_eq!(create.id, read.id);
        assert_eq!(create.account_id, read.account_id);
        assert_eq!(create.pem, read.pem);
        assert_eq!(create.fingerprint, read.fingerprint);
        Ok(())
    }

    #[tokio::test]
    async fn creat_read_all() -> Result<(), RequestableError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        let device_api_key = DeviceApiKey::new(pem);
        let create = device_api_key.create(&mut client).await?;
        let read_all = DeviceApiKey::read_all(&mut client).await?;
        assert!(read_all.len() == 2);
        assert_eq!(create.id, read_all[1].id);
        assert_eq!(create.account_id, read_all[1].account_id);
        assert_eq!(create.pem, read_all[1].pem);
        assert_eq!(create.fingerprint, read_all[1].fingerprint);
        Ok(())
    }

    #[tokio::test]
    async fn create_delete() -> Result<(), RequestableError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        let device_api_key = DeviceApiKey::new(pem);
        let create = device_api_key.create(&mut client).await?;
        let delete = DeviceApiKey::delete(&mut client, &create.id().unwrap().to_string()).await?;
        assert_eq!(create.id, delete.id);
        assert_eq!(create.account_id, delete.account_id);
        assert_eq!(create.fingerprint, delete.fingerprint);
        Ok(())
    }
}