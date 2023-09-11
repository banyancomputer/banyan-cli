use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan_api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct ReadBucketKey {
    pub bucket_id: Uuid,
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ReadAllBucketKeys {
    pub bucket_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ReadBucketKeyResponse {
    pub id: Uuid,
    pub approved: bool,
    pub pem: String,
    pub fingerprint: String,
}

#[derive(Debug, Deserialize)]
pub struct ReadAllBucketKeysResponse(pub(crate) Vec<ReadBucketKeyResponse>);

impl ApiRequest for ReadBucketKey {
    type ResponseType = ReadBucketKeyResponse;
    type ErrorType = ReadBucketKeyError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/keys/{}", self.bucket_id, self.id);
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

impl ApiRequest for ReadAllBucketKeys {
    type ResponseType = ReadAllBucketKeysResponse;
    type ErrorType = ReadBucketKeyError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/keys", self.bucket_id);
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ReadBucketKeyError {
    #[serde(rename = "error")]
    kind: ReadBucketKeyErrorKind,
}

impl Display for ReadBucketKeyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ReadBucketKeyErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the bucket",
        };

        f.write_str(msg)
    }
}

impl Error for ReadBucketKeyError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReadBucketKeyErrorKind {
    Unknown,
}
