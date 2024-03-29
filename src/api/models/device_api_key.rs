use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::{
    client::Client,
    error::ApiError,
    requests::core::auth::device_api_key::{
        create::{CreateDeviceApiKey, CreateDeviceApiKeyResponse},
        delete::DeleteDeviceApiKey,
        read::{
            ReadAllDeviceApiKeys, ReadAllDeviceApiKeysResponse, ReadDeviceApiKey,
            ReadDeviceApiKeyResponse,
        },
    },
};

#[derive(Debug, Deserialize, Serialize, Clone)]
/// DeviceApiKey Definition
pub struct DeviceApiKey {
    /// The unique identifier for the device api key
    pub id: Uuid,
    /// The public key material for the device api key
    pub pem: String,
    /// The fingerprint of the device api key
    pub fingerprint: String,
}

impl DeviceApiKey {
    /// Create a new instance of this model or data structure. Attaches the associated credentials to the client.
    pub async fn create(pem: String, client: &mut Client) -> Result<Self, ApiError> {
        let response: CreateDeviceApiKeyResponse =
            client.call(CreateDeviceApiKey { pem: pem.clone() }).await?;
        Ok(Self {
            id: response.id,
            fingerprint: response.fingerprint,
            pem,
        })
    }

    /// Read all instances of this model or data structure.
    pub async fn read_all(client: &mut Client) -> Result<Vec<Self>, ApiError> {
        let response: ReadAllDeviceApiKeysResponse = client.call(ReadAllDeviceApiKeys).await?;
        // Map the response to the model
        let mut device_api_keys = Vec::new();
        for device_api_key in response.0 {
            device_api_keys.push(Self {
                id: device_api_key.id,
                fingerprint: device_api_key.fingerprint,
                pem: device_api_key.pem,
            });
        }
        Ok(device_api_keys)
    }

    /// Get the account associated with the current credentials. You do not need to pass an ID for this request.
    pub async fn read(client: &mut Client, id: Uuid) -> Result<Self, ApiError> {
        let response: ReadDeviceApiKeyResponse = client.call(ReadDeviceApiKey { id }).await?;
        Ok(Self {
            id: response.id,
            fingerprint: response.fingerprint,
            pem: response.pem,
        })
    }

    /// Delete the device api key from the account
    pub async fn delete(self, client: &mut Client) -> Result<(), ApiError> {
        client
            .call_no_content(DeleteDeviceApiKey { id: self.id })
            .await
    }

    /// Delete the device api key from the account by id
    pub async fn delete_by_id(client: &mut Client, id: Uuid) -> Result<(), ApiError> {
        client.call_no_content(DeleteDeviceApiKey { id }).await
    }
}

// TODO: wasm tests

#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod test {
    use crate::api::{
        error::ApiError,
        models::{account::test::authenticated_client, device_api_key::DeviceApiKey},
        utils::generate_api_key,
    };

    #[tokio::test]
    async fn create() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        println!("pem: {:?}", pem);
        let _ = DeviceApiKey::create(pem, &mut client).await?;
        Ok(())
    }

    #[tokio::test]
    async fn create_read() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        let create = DeviceApiKey::create(pem, &mut client).await?;
        let read = DeviceApiKey::read(&mut client, create.id).await?;
        assert_eq!(create.id, read.id);
        assert_eq!(create.pem, read.pem);
        assert_eq!(create.fingerprint, read.fingerprint);
        Ok(())
    }

    #[tokio::test]
    async fn create_read_all() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        let create = DeviceApiKey::create(pem, &mut client).await?;
        let read_all = DeviceApiKey::read_all(&mut client).await?;
        assert!(!read_all.is_empty());
        assert_eq!(create.id, read_all[1].id);
        assert_eq!(create.pem, read_all[1].pem);
        assert_eq!(create.fingerprint, read_all[1].fingerprint);
        Ok(())
    }

    #[tokio::test]
    async fn create_delete() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        let create = DeviceApiKey::create(pem, &mut client).await?;
        create.clone().delete(&mut client).await?;
        let all_remaining = DeviceApiKey::read_all(&mut client).await?;
        assert!(!all_remaining.iter().any(|value| value.id == create.id));
        Ok(())
    }
}
