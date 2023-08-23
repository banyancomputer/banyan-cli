use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan_api::{
    client::Client,
    error::ClientError,
    requests::auth::device_api_key::{create::*, delete::*, read::*},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
/// DeviceApiKey Definition
pub struct DeviceApiKey {
    /// The unique identifier for the device api key
    pub id: Uuid,
    /// The fingerprint of the device api key
    pub fingerprint: String,
    /// The public key material for the device api key
    pub pem: String,
}

impl DeviceApiKey {
    /// Create a new instance of this model or data structure. Attaches the associated credentials to the client.
    pub async fn create(pem: String, client: &mut Client) -> Result<Self, ClientError> {
        let response: CreateDeviceApiKeyResponse =
            client.call(CreateDeviceApiKey { pem: pem.clone() }).await?;
        Ok(Self {
            id: response.id,
            fingerprint: response.fingerprint,
            pem,
        })
    }

    /// Read all instances of this model or data structure.
    pub async fn read_all(client: &mut Client) -> Result<Vec<Self>, ClientError> {
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
    pub async fn read(client: &mut Client, id: Uuid) -> Result<Self, ClientError> {
        let response: ReadDeviceApiKeyResponse = client.call(ReadDeviceApiKey { id }).await?;
        Ok(Self {
            id: response.id,
            fingerprint: response.fingerprint,
            pem: response.pem,
        })
    }

    /// Delete the device api key from the account
    pub async fn delete(self, _client: &mut Client) -> Result<String, ClientError> {
        let response: DeleteDeviceApiKeyResponse =
            _client.call(DeleteDeviceApiKey { id: self.id }).await?;
        Ok(response.id.to_string())
    }

    /// Delete the device api key from the account by id
    pub async fn delete_by_id(client: &mut Client, id: Uuid) -> Result<String, ClientError> {
        let response: DeleteDeviceApiKeyResponse = client.call(DeleteDeviceApiKey { id }).await?;
        Ok(response.id.to_string())
    }
}

// TODO: wasm tests

#[cfg(test)]

mod test {
    use super::*;
    use crate::banyan_api::models::account::test::authenticated_client;
    use crate::banyan_api::utils::generate_api_key;

    #[tokio::test]
    async fn create() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        println!("pem: {:?}", pem);
        let _ = DeviceApiKey::create(pem, &mut client).await?;
        Ok(())
    }

    #[tokio::test]
    async fn create_read() -> Result<(), ClientError> {
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
    async fn create_read_all() -> Result<(), ClientError> {
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
    async fn create_delete() -> Result<(), ClientError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        let create = DeviceApiKey::create(pem, &mut client).await?;
        let delete = create.clone().delete(&mut client).await?;
        assert_eq!(create.id.to_string(), delete);
        Ok(())
    }
}
