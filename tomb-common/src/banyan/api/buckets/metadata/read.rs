use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan::api::ApiRequest;
use crate::banyan::models::metadata::MetadataState;

#[derive(Debug, Serialize)]
pub struct ReadMetadata {
    pub bucket_id: Uuid,
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ReadAllMetadata {
    pub bucket_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ReadCurrentMetadata {
    pub bucket_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ReadMetadataResponse {
    pub id: Uuid,
    pub root_cid: String,
    pub metadata_cid: String,
    pub data_size: i64,
    pub state: MetadataState,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct ReadAllMetadataResponse(pub(crate) Vec<ReadMetadataResponse>);

impl ApiRequest for ReadMetadata {
    type ResponseType = ReadMetadataResponse;
    type ErrorType = ReadMetadataError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/metadata/{}", self.bucket_id, self.id);
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

impl ApiRequest for ReadAllMetadata {
    type ResponseType = ReadAllMetadataResponse;
    type ErrorType = ReadMetadataError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/metadata", self.bucket_id);
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

impl ApiRequest for ReadCurrentMetadata {
    type ResponseType = ReadMetadataResponse;
    type ErrorType = ReadMetadataError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/metadata/current", self.bucket_id);
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ReadMetadataError {
    #[serde(rename = "error")]
    kind: ReadMetadataErrorKind,
}

impl Display for ReadMetadataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ReadMetadataErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the bucket",
        };

        f.write_str(msg)
    }
}

impl Error for ReadMetadataError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReadMetadataErrorKind {
    Unknown,
}
