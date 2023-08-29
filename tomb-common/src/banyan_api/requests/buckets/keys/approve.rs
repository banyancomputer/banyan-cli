use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan_api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct ApproveBucketKey {
    pub bucket_id: Uuid,
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ApproveBucketKeyResponse {
    pub id: Uuid,
    pub approved: bool,
    pub pem: String,
}

impl ApiRequest for ApproveBucketKey {
    type ResponseType = ApproveBucketKeyResponse;
    type ErrorType = ApproveBucketKeyError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/keys/{}/approve", self.bucket_id, self.id);
        let full_url = base_url.join(&path).unwrap();
        client.post(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ApproveBucketKeyError {
    #[serde(rename = "error")]
    kind: ApproveBucketKeyErrorKind,
}

impl Display for ApproveBucketKeyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ApproveBucketKeyErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the bucket",
        };

        f.write_str(msg)
    }
}

impl Error for ApproveBucketKeyError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum ApproveBucketKeyErrorKind {
    Unknown,
}
