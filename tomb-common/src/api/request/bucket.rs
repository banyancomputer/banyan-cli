use super::{RequestMetadata, Requestable};
use crate::api::error::StatusError;
use clap::{Args, Subcommand};
use reqwest::Method;
use serde::{Deserialize, Serialize};

const API_PREFIX: &str = "/api/v1/buckets";

/// Bucket Request
#[derive(Clone, Debug, Serialize, Subcommand)]
pub enum BucketRequest {
    /// Create a Bucket
    Create(CreateBucketRequest),
    /// List a Bucket
    List(ListBucketRequest),
    /// Get a Bucket
    Get(GetBucketRequest),
    /// Delete a Bucket
    Delete(DeleteBucketRequest),
}

/// Request to create a Bucket
#[derive(Clone, Debug, Serialize, Args)]
pub struct CreateBucketRequest {
    /// Bucket Name
    #[serde(rename = "friendly_name")]
    pub name: String,
    /// Bucket Type
    pub r#type: BucketType,
    /// Bucket Public Key
    pub initial_public_key: String,
}

/// Possible types of Bucket
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, clap::clap_derive::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum BucketType {
    /// Storage only
    Backup,
    /// Syncable
    Interactive,
}

/// Request to list all Buckets
#[derive(Clone, Debug, Serialize, Args)]
pub struct ListBucketRequest;

/// Request to get a Bucket
#[derive(Clone, Debug, Serialize, Args)]
pub struct GetBucketRequest {
    /// Bucket Id
    pub bucket_id: String,
}

/// Request to delete a Bucket
#[derive(Clone, Debug, Serialize, Args)]
pub struct DeleteBucketRequest {
    /// Bucket Id
    pub bucket_id: String,
}

impl Requestable for CreateBucketRequest {
    type ErrorType = StatusError;
    type ResponseType = BucketResponse;

    fn metadata(&self) -> RequestMetadata {
        RequestMetadata {
            endpoint: API_PREFIX.to_string(),
            method: Method::POST,
            auth: true,
        }
    }
}

impl Requestable for ListBucketRequest {
    type ErrorType = StatusError;
    type ResponseType = ListBucketResponse;
    fn metadata(&self) -> RequestMetadata {
        RequestMetadata {
            endpoint: API_PREFIX.to_string(),
            method: Method::GET,
            auth: true,
        }
    }
}

impl Requestable for GetBucketRequest {
    type ErrorType = StatusError;
    type ResponseType = BucketResponse;

    fn metadata(&self) -> RequestMetadata {
        RequestMetadata {
            endpoint: format!("{}/{}", API_PREFIX, self.bucket_id),
            method: Method::GET,
            auth: true,
        }
    }
}

impl Requestable for DeleteBucketRequest {
    type ErrorType = StatusError;
    type ResponseType = BucketResponse;

    fn metadata(&self) -> RequestMetadata {
        RequestMetadata {
            endpoint: format!("{}/{}", API_PREFIX, self.bucket_id),
            method: Method::DELETE,
            auth: true,
        }
    }
}

/// Response from requesting Bucket
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct BucketResponse {
    /// Bucket ID
    pub id: String,
    /// Bucket Name
    pub friendly_name: String,
    /// Bucket Type
    pub r#type: BucketType,
}

/// Response from requesting list of Buckets
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ListBucketResponse(Vec<BucketResponse>);

#[cfg(test)]
mod test {
    use crate::api::{
        error::ClientError,
        request::{
            fake::*, BucketType, CreateBucketRequest, DeleteBucketRequest, GetBucketRequest,
            ListBucketRequest,
        },
    };
    use serial_test::serial;
    use tomb_crypt::prelude::WrappingPublicKey;

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

    #[tokio::test]
    #[serial]
    async fn create_delete() -> Result<(), ClientError> {
        let (mut client, _, public_key) = setup().await?;
        let public_pem = String::from_utf8(public_key.export().await.unwrap()).unwrap();
        let request = CreateBucketRequest {
            name: "test interactive bucket".to_string(),
            r#type: BucketType::Interactive,
            initial_public_key: public_pem,
        };
        let response1 = client.send(request.clone()).await?;
        let bucket_id = response1.clone().id;

        let response2 = client.send(DeleteBucketRequest { bucket_id }).await;

        assert!(response2.is_err());

        Ok(())
    }
}
