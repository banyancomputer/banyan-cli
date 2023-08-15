use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan::api::auth::device_api_key::{create::*, delete::*, read::*};
use crate::banyan::client::Client;
use crate::banyan::models::ModelError;

#[derive(Debug, Deserialize, Serialize, Clone)]
/// DeviceApiKey Definition. Note the options for the fields.
/// This does not apply to the Db models. It just allows working with structs as partials.
pub struct DeviceApiKey {
    pub id: Uuid,
    pub fingerprint: String,
    pub pem: String,
}

impl DeviceApiKey {
    /// Create a new instance of this model or data structure. Attaches the associated credentials to the client.
    async fn create(pem: String, client: &mut Client) -> Result<Self, ModelError> {
        let response: CreateDeviceApiKeyResponse = client
            .call(CreateDeviceApiKey { pem: pem.clone() })
            .await
            .map_err(|_| ModelError::client_error())?;
        Ok(Self {
            id: response.id,
            fingerprint: response.fingerprint,
            pem,
        })
    }

    /// Read all instances of this model or data structure.
    async fn read_all(client: &mut Client) -> Result<Vec<Self>, ModelError> {
        let response: ReadAllDeviceApiKeysResponse = client
            .call(ReadAllDeviceApiKeys)
            .await
            .map_err(|_| ModelError::client_error())?;
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
    async fn read(client: &mut Client, id: Uuid) -> Result<Self, ModelError> {
        let response: ReadDeviceApiKeyResponse = client
            .call(ReadDeviceApiKey { id: id.clone() })
            .await
            .map_err(|_| ModelError::client_error())?;
        Ok(Self {
            id: response.id,
            fingerprint: response.fingerprint,
            pem: response.pem,
        })
    }

    async fn delete(self: Self, _client: &mut Client) -> Result<String, ModelError> {
        let response: DeleteDeviceApiKeyResponse = _client
            .call(DeleteDeviceApiKey { id: self.id })
            .await
            .map_err(|_| ModelError::client_error())?;
        Ok(response.id.to_string())
    }

    async fn delete_by_id(client: &mut Client, id: Uuid) -> Result<String, ModelError> {
        let response: DeleteDeviceApiKeyResponse = client
            .call(DeleteDeviceApiKey { id: id.clone() })
            .await
            .map_err(|_| ModelError::client_error())?;
        Ok(response.id.to_string())
    }
}

// TODO: wasm tests

#[cfg(test)]
mod test {
    use super::*;
    use crate::banyan::models::account::generate_api_key;
    use crate::banyan::models::account::test::authenticated_client;

    #[tokio::test]
    async fn create() -> Result<(), ModelError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        println!("pem: {:?}", pem);
        let create = DeviceApiKey::create(pem, &mut client).await?;
        Ok(())
    }

    #[tokio::test]
    async fn create_read() -> Result<(), ModelError> {
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
    async fn creat_read_all() -> Result<(), ModelError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        let create = DeviceApiKey::create(pem, &mut client).await?;
        let read_all = DeviceApiKey::read_all(&mut client).await?;
        assert!(read_all.len() > 0);
        assert_eq!(create.id, read_all[1].id);
        assert_eq!(create.pem, read_all[1].pem);
        assert_eq!(create.fingerprint, read_all[1].fingerprint);
        Ok(())
    }

    #[tokio::test]
    async fn create_delete() -> Result<(), ModelError> {
        let mut client = authenticated_client().await;
        let (_, pem) = generate_api_key().await;
        let create = DeviceApiKey::create(pem, &mut client).await?;
        let delete = create.clone().delete(&mut client).await?;
        assert_eq!(create.id.to_string(), delete);
        Ok(())
    }
}
