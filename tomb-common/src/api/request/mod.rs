use std::{error::Error, fmt::Display};
use async_trait::async_trait;
use reqwest::{Method, Response};
use serde::{Serialize, de::DeserializeOwned, Deserialize};
use uuid::Uuid;

mod bucket;
mod key;
mod metadata;

#[cfg(test)]
mod fake;

pub use bucket::*;
pub use key::*;
pub use metadata::*;

#[async_trait(?Send)]
pub trait Requestable: Serialize + Sized {
    type ErrorType: DeserializeOwned + Error + Send + Sync + 'static;
    type ResponseType: Respondable<Self, Self::ErrorType>;

    // Obtain the url suffix of the endpoint
    fn endpoint(&self) -> String;
    fn method(&self) -> Method;
    fn authed(&self) -> bool;
}

#[async_trait(?Send)]
pub trait Respondable<R: Requestable, E: Error>: Sized {
    async fn process(request: R, response: Response) -> Result<Self, E>;
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
    use tomb_crypt::prelude::{EcEncryptionKey, WrappingPrivateKey, WrappingPublicKey};

    use crate::api::{client::Client, request::{BucketRequest, CreateBucketRequest, fake::*}, error::ClientError};

    const TEST_REMOTE: &str = "http://127.0.0.1:3001/";


    #[tokio::test]
    async fn fake() -> Result<(), ClientError> { 
        let client = Client::new(TEST_REMOTE).unwrap();
        let response: FakeResponse = client.send(FakeRequest::RegisterAccount).await?;
        // Create a local key pair
        let private_key = EcEncryptionKey::generate().await.unwrap();
        let public_key = private_key.public_key().unwrap();
        // Represent as PEM string
        let public_key_pem_string = String::from_utf8(public_key.export().await.unwrap()).unwrap();
        // Send a device registration request
        let response: FakeResponse = client.send(FakeRequest::RegisterDeviceKey(FakeRegisterDeviceKeyRequest { public_key: public_key_pem_string })).await?;

        println!("fake response: {:?}", response);

        Ok(())
    }

    #[tokio::test]
    async fn create() -> Result<(), ClientError> {
        let client = Client::new(TEST_REMOTE).unwrap();
        let request = BucketRequest::Create(CreateBucketRequest {
            name: "silly name :3".to_string()
        });
        let response = client.send(request).await?;
        println!("response is: {:?}", response);
        Ok(())
    }

    // #[tokio::test]
    // async fn create_get() -> Result<(), ClientError> {
    //     let mut api_client = fake_authenticated_client().await;
    //     let friendly_name = "test interactive bucket".to_string();
    //     let response1 = api_client.call(CreateBucket {
    //         friendly_name: friendly_name.clone(),
    //         r#type: BucketType::Interactive,
    //         initial_public_key: "ECDH public key pem formatted bits".to_string(),
    //     }).await?;

    //     let response2 = api_client.call(GetBucket {
    //         bucket_id: response1.id,
    //     }).await?;

    //     assert_eq!(response1, response2);
    //     Ok(())
    // }

    // #[tokio::test]
    // async fn list() -> Result<(), ClientError> {
    //     let mut api_client = fake_authenticated_client().await;
    //     let buckets = api_client.call(ListBuckets{

    //     }).await?;

    //     println!("{buckets:?}");

    //     Ok(())
    // }
}