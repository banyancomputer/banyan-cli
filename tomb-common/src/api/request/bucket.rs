use crate::api::error::{ClientError, StatusError};
use clap::{Args, Subcommand};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt::Display, str::FromStr};

use super::Requestable;

const API_PREFIX: &str = "/api/v1/buckets";

#[derive(Clone, Debug, Serialize, Subcommand)]
pub enum BucketRequest {
    Create(CreateBucketRequest),
    List(ListBucketRequest),
    Get(GetBucketRequest),
    Delete(DeleteBucketRequest),
}

#[derive(Clone, Debug, Serialize, Args)]
pub struct CreateBucketRequest {
    #[serde(rename = "friendly_name")]
    pub name: String,
    pub r#type: BucketType,
    pub initial_public_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, clap::clap_derive::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum BucketType {
    Backup,
    Interactive,
}

#[derive(Clone, Debug, Serialize, Args)]
pub struct ListBucketRequest;
#[derive(Clone, Debug, Serialize, Args)]
pub struct GetBucketRequest {
    pub bucket_id: String,
}
#[derive(Clone, Debug, Serialize, Args)]
pub struct DeleteBucketRequest {
    pub bucket_id: String,
}

impl Requestable for CreateBucketRequest {
    type ErrorType = StatusError;
    type ResponseType = BucketResponse;
    fn endpoint(&self) -> String {
        format!("{}", API_PREFIX)
    }
    fn method(&self) -> Method {
        Method::POST
    }
    fn authed(&self) -> bool {
        true
    }
}

impl Requestable for ListBucketRequest {
    type ErrorType = StatusError;
    type ResponseType = ListBucketResponse;
    fn endpoint(&self) -> String {
        format!("{}", API_PREFIX)
    }
    fn method(&self) -> Method {
        Method::GET
    }
    fn authed(&self) -> bool {
        true
    }
}

impl Requestable for GetBucketRequest {
    type ErrorType = StatusError;
    type ResponseType = BucketResponse;
    fn endpoint(&self) -> String {
        format!("{}/{}", API_PREFIX, self.bucket_id)
    }
    fn method(&self) -> Method {
        Method::GET
    }
    fn authed(&self) -> bool {
        true
    }
}

impl Requestable for DeleteBucketRequest {
    type ErrorType = StatusError;
    type ResponseType = BucketResponse;
    fn endpoint(&self) -> String {
        format!("{}/{}", API_PREFIX, self.bucket_id)
    }
    fn method(&self) -> Method {
        Method::DELETE
    }
    fn authed(&self) -> bool {
        true
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct BucketResponse {
    pub id: String,
    pub friendly_name: String,
    pub r#type: BucketType,
}

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
