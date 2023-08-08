use std::{error::Error, fmt::Display};
use reqwest::Method;
use serde::{Serialize, de::DeserializeOwned, Deserialize};
use uuid::Uuid;

pub trait Requestable: Serialize {
    type ResponseType: DeserializeOwned;
    type ErrorType: DeserializeOwned + Error + Send + Sync + 'static;

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

#[derive(Clone, Debug, Serialize)]
pub enum BucketRequest {
    Create(CreateBucketRequest),
    List,
    Get(Uuid),
    Delete(Uuid),
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateBucketRequest {
    name: String
}

#[derive(Clone, Debug, Deserialize)]
pub enum BucketError {
    #[serde(rename = "status")]
    Any(String)
}

impl Display for BucketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unknown")
    }
}

impl Error for BucketError {}

#[derive(Clone, Debug, Deserialize)]
pub enum BucketResponse {
    // #[serde(flatten)]
    Create,
    List,
    Get,
    Delete
}

impl Requestable for BucketRequest {
    type ResponseType = BucketResponse;
    type ErrorType = BucketError;

    fn endpoint(&self) -> String {
        match self {
            BucketRequest::Create(_) | BucketRequest::List => format!("{}/buckets", API_PREFIX),
            BucketRequest::Get(uuid) | BucketRequest::Delete(uuid) => format!("/buckets/{}", uuid),
        }
    }

    fn method(&self) -> Method {
        match self {
            BucketRequest::Create(_) => Method::POST,
            BucketRequest::List | BucketRequest::Get(_) => Method::GET,
            BucketRequest::Delete(_) => Method::DELETE,
        }
    }

    fn authed(&self) -> bool { true }
}

#[derive(Clone, Debug)]
pub enum KeyRequest {
    Create {
        
    },
    Get {

    },
    Delete {

    },
}

#[derive(Clone, Debug)]
pub enum MetadataRequest {
    Create {
        
    },
    Get {

    },
    Delete {

    },
}


#[cfg(test)]
mod test {
    use crate::api::{client::Client, request::{BucketRequest, CreateBucketRequest}, error::ClientError};

    const TEST_REMOTE: &str = "http://127.0.0.1:3001/";


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