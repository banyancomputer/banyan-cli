use std::{error::Error, fmt::Display};

use async_trait::async_trait;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Requestable, Respondable};

const API_PREFIX: &str = "/api/v1/buckets";

#[derive(Clone, Debug, Serialize)]
pub enum BucketRequest {
    Create(CreateBucketRequest),
    List,
    Get(Uuid),
    Delete(Uuid),
}

#[derive(Clone, Debug, Serialize)]
pub struct CreateBucketRequest {
    pub name: String,
}

impl Requestable for BucketRequest {
    fn endpoint(&self) -> String {
        match self {
            BucketRequest::Create(_) | BucketRequest::List => format!("{}", API_PREFIX),
            BucketRequest::Get(uuid) | BucketRequest::Delete(uuid) => {
                format!("{}{}", API_PREFIX, uuid)
            }
        }
    }

    fn method(&self) -> Method {
        match self {
            BucketRequest::Create(_) => Method::POST,
            BucketRequest::List | BucketRequest::Get(_) => Method::GET,
            BucketRequest::Delete(_) => Method::DELETE,
        }
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

#[derive(Clone, Debug, Deserialize)]
pub enum BucketResponse {
    Create,
    List,
    Get,
    Delete,
}

#[async_trait(?Send)]
impl Respondable<BucketRequest, BucketError> for BucketResponse {
    async fn process(
        request: BucketRequest,
        response: reqwest::Response,
    ) -> Result<Self, BucketError> {
        match request {
            BucketRequest::Create(_) => todo!(),
            BucketRequest::List => todo!(),
            BucketRequest::Get(_) => todo!(),
            BucketRequest::Delete(_) => todo!(),
        }
    }
}
