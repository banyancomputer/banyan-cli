use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::{models::metadata::MetadataState, requests::ApiRequest};

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
    pub previous_cid: Option<String>,
    pub data_size: i64,
    pub state: MetadataState,
    pub created_at: i64,
    pub updated_at: i64,
    pub snapshot_id: Option<Uuid>,
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
pub struct ReadMetadataError {
    msg: String,
}

impl Display for ReadMetadataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for ReadMetadataError {}
