use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan_api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct RejectBucketKey {
    pub bucket_id: Uuid,
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RejectBucketKeyResponse {
    pub id: Uuid,
    pub approved: bool,
}

impl ApiRequest for RejectBucketKey {
    type ResponseType = RejectBucketKeyResponse;
    type ErrorType = RejectBucketKeyError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/keys/{}/reject", self.bucket_id, self.id);
        let full_url = base_url.join(&path).unwrap();
        client.post(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct RejectBucketKeyError {
    #[serde(rename = "error")]
    kind: RejectBucketKeyErrorKind,
}

impl Display for RejectBucketKeyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use RejectBucketKeyErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred rejecting the bucket",
        };

        f.write_str(msg)
    }
}

impl Error for RejectBucketKeyError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum RejectBucketKeyErrorKind {
    Unknown,
}
