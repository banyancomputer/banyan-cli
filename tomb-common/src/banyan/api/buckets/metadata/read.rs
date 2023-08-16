use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan::models::bucket_metadata::BucketMetadataState;
use crate::banyan::api::ApiRequest;

#[derive(Debug, Serialize)]
pub struct ReadBucketMetadata {
    pub bucket_id: Uuid,
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ReadAllBucketMetadata {
    pub bucket_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ReadBucketMetadataResponse {
    pub id: Uuid,
    pub root_cid: String,
    pub metadata_cid: String,
    pub data_size: i64,
    pub state: BucketMetadataState,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct ReadAllBucketMetadataResponse(pub(crate) Vec<ReadBucketMetadataResponse>);

impl ApiRequest for ReadBucketMetadata {
    type ResponseType = ReadBucketMetadataResponse;
    type ErrorType = ReadBucketMetadataError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/metadata/{}", self.bucket_id, self.id);
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

impl ApiRequest for ReadAllBucketMetadata {
    type ResponseType = ReadAllBucketMetadataResponse;
    type ErrorType = ReadBucketMetadataError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/metadata", self.bucket_id);
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ReadBucketMetadataError {
    #[serde(rename = "error")]
    kind: ReadBucketMetadataErrorKind,
}

impl Display for ReadBucketMetadataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ReadBucketMetadataErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the bucket",
        };

        f.write_str(msg)
    }
}

impl Error for ReadBucketMetadataError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReadBucketMetadataErrorKind {
    Unknown,
}
