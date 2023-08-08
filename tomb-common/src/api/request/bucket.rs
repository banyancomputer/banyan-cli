use std::{convert::Infallible, error::Error, fmt::Display};

use async_trait::async_trait;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::error::InfallibleError;

use super::Requestable;

const API_PREFIX: &str = "/api/v1/buckets";

#[derive(Clone, Debug, Serialize)]
pub enum BucketRequest {
    Create(CreateBucketRequest),
    List(ListBucketRequest),
    Get(GetBucketRequest),
    Delete(DeleteBucketRequest),
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateBucketRequest {
    #[serde(rename = "friendly_name")]
    pub name: String,
    pub r#type: BucketType,
    pub initial_public_key: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BucketType {
    Backup,
    Interactive,
}

#[derive(Clone, Debug, Serialize)]
pub struct ListBucketRequest;
#[derive(Clone, Debug, Serialize)]
pub struct GetBucketRequest {
    pub bucket_id: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct DeleteBucketRequest {
    pub bucket_id: String,
}

impl Requestable for CreateBucketRequest {
    type ErrorType = BucketError;
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
    type ErrorType = BucketError;
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
    type ErrorType = BucketError;
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
    type ErrorType = BucketError;
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

#[derive(Clone, Debug, Deserialize)]
pub enum BucketError {
    #[serde(rename = "status")]
    Any(String),
}

impl Display for BucketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unknown")
    }
}

impl Error for BucketError {}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct BucketResponse {
    pub id: String,
    pub friendly_name: String,
    pub r#type: BucketType,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ListBucketResponse(Vec<BucketResponse>);
