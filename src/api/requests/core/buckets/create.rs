use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::{
    models::bucket::{BucketType, StorageClass},
    requests::{core::buckets::keys::create::CreateBucketKeyResponse, ApiRequest},
};

#[derive(Debug, Serialize)]
pub struct CreateBucket {
    pub name: String,
    pub r#type: BucketType,
    pub storage_class: StorageClass,
    pub initial_bucket_key_pem: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateBucketResponse {
    pub id: Uuid,
    pub name: String,
    pub r#type: BucketType,
    pub storage_class: StorageClass,
    pub initial_bucket_key: CreateBucketKeyResponse,
}

impl ApiRequest for CreateBucket {
    type ResponseType = CreateBucketResponse;
    type ErrorType = CreateBucketError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v1/buckets").unwrap();
        client.post(full_url).json(&self)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct CreateBucketError {
    msg: String,
}

impl Display for CreateBucketError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for CreateBucketError {}
