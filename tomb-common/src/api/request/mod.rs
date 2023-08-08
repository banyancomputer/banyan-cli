use async_trait::async_trait;
use reqwest::{Method, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{error::Error, fmt::Display};
use uuid::Uuid;

mod bucket;
mod key;
mod metadata;
mod who;

#[cfg(test)]
mod fake;

pub use bucket::*;
pub use key::*;
pub use metadata::*;
pub use who::*;

#[async_trait(?Send)]
pub trait Requestable: Serialize + Sized {
    type ErrorType: DeserializeOwned + Error + Send + Sync + 'static;
    type ResponseType: DeserializeOwned;

    // Obtain the url suffix of the endpoint
    fn endpoint(&self) -> String;
    fn method(&self) -> Method;
    fn authed(&self) -> bool;
}

const API_PREFIX: &str = "/api/v1";

#[derive(Clone, Debug)]
pub enum Request {
    /// Set the remote endpoint where buckets are synced to / from
    Bucket(BucketRequest),
    /// Set the remote endpoint where buckets are synced to / from
    Keys(KeyRequest),
    /// Set the remote endpoint where buckets are synced to / from
    Metadata(MetadataRequest),
}

#[cfg(test)]
mod test {
    use jsonwebtoken::{get_current_timestamp, EncodingKey};
    use serial_test::serial;
    use tomb_crypt::{
        prelude::{EcEncryptionKey, EcPublicEncryptionKey, WrappingPrivateKey, WrappingPublicKey},
        pretty_fingerprint,
    };

    use crate::api::{
        client::Client,
        credentials::Credentials,
        error::{ClientError, InfallibleError},
        request::{
            fake::*, BucketRequest, BucketType, CreateBucketRequest, GetBucketRequest,
            ListBucketRequest, WhoRequest,
        },
        token::Token,
    };

    const TEST_REMOTE: &str = "http://127.0.0.1:3001/";

    async fn setup() -> Result<(Client, EcEncryptionKey, EcPublicEncryptionKey), ClientError> {
        let mut client = Client::new(TEST_REMOTE).unwrap();
        // Register
        let account_response = client.send(RegisterAccountRequest).await?;
        // Create a local key pair
        let private_key = EcEncryptionKey::generate().await.unwrap();
        let public_key = private_key.public_key().unwrap();
        // Represent as PEM string
        let public_key_pem_string = String::from_utf8(public_key.export().await.unwrap()).unwrap();
        // Set the bearer token
        client.bearer_token = Some((
            get_current_timestamp() + 870,
            account_response.token.clone(),
        ));
        // Assert that it's accessible
        assert!(client.bearer_token().is_some());

        //
        let jwt_signing_key =
            EncodingKey::from_ec_pem(&private_key.export().await.unwrap()).unwrap();

        // Update the credentials
        client.credentials = Some(Credentials {
            account_id: account_response.id,
            fingerprint: pretty_fingerprint(&public_key.fingerprint().await.unwrap()),
            signing_key: jwt_signing_key,
        });

        // Send a device registration request
        let device_response = client
            .send(RegisterDeviceKeyRequest {
                token: account_response.token,
                public_key: public_key_pem_string,
            })
            .await?;

        // Empty out the broken bearer token
        client.bearer_token = None;

        let authenticated_info = client.send(WhoRequest).await?;
        assert_eq!(authenticated_info.account_id, device_response.account_id);

        Ok((client, private_key, public_key))
    }

    #[tokio::test]
    #[serial]
    async fn create() -> Result<(), ClientError> {
        let (mut client, _, public_key) = setup().await?;
        let public_pem = String::from_utf8(public_key.export().await.unwrap()).unwrap();
        let request = CreateBucketRequest {
            name: "test interactive bucket".to_string(),
            r#type: BucketType::Interactive,
            initial_public_key: public_pem,
        };
        let response = client.send(request.clone()).await?;

        assert_eq!(request.name, response.friendly_name);
        assert_eq!(request.r#type, response.r#type);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn create_get() -> Result<(), ClientError> {
        let (mut client, _, public_key) = setup().await?;
        let public_pem = String::from_utf8(public_key.export().await.unwrap()).unwrap();
        let request = CreateBucketRequest {
            name: "test interactive bucket".to_string(),
            r#type: BucketType::Interactive,
            initial_public_key: public_pem,
        };
        let response1 = client.send(request.clone()).await?;
        let response2 = client
            .send(GetBucketRequest {
                bucket_id: response1.clone().id,
            })
            .await?;

        assert_eq!(response1, response2);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn list() -> Result<(), ClientError> {
        let (mut client, _, _) = setup().await?;
        let _ = client.send(ListBucketRequest).await?;
        Ok(())
    }
}
