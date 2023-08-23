use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan_api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct DeleteBucket {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct DeleteBucketResponse {
    pub id: Uuid,
    pub name: String,
}

impl ApiRequest for DeleteBucket {
    type ResponseType = DeleteBucketResponse;
    type ErrorType = DeleteBucketError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}", self.id);
        let url = base_url.join(&path).unwrap();
        client.delete(url).json(&self)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct DeleteBucketError {
    #[serde(rename = "error")]
    kind: DeleteBucketErrorKind,
}

impl Display for DeleteBucketError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use DeleteBucketErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred deleting the bucket",
        };

        f.write_str(msg)
    }
}

impl Error for DeleteBucketError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum DeleteBucketErrorKind {
    Unknown,
}
