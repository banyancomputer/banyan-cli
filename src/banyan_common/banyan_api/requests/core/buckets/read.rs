use std::error::Error;

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan_common::banyan_api::models::bucket::{BucketType, StorageClass};
use crate::banyan_common::banyan_api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct ReadBucket {
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ReadAllBuckets;

#[derive(Debug, Deserialize)]
pub struct ReadBucketResponse {
    pub id: Uuid,
    pub name: String,
    pub r#type: BucketType,
    pub storage_class: StorageClass,
}

#[derive(Debug, Deserialize)]
pub struct ReadAllBucketsResponse(pub(crate) Vec<ReadBucketResponse>);

impl ApiRequest for ReadBucket {
    type ErrorType = ReadBucketError;
    type ResponseType = ReadBucketResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let bucket_id = self.id.to_string();
        let full_url = base_url
            .join(format!("/api/v1/buckets/{}", bucket_id).as_str())
            .unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

impl ApiRequest for ReadAllBuckets {
    type ErrorType = ReadBucketError;
    type ResponseType = ReadAllBucketsResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v1/buckets").unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct ReadBucketError {
    msg: String,
}

impl Error for ReadBucketError {}

impl std::fmt::Display for ReadBucketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}
