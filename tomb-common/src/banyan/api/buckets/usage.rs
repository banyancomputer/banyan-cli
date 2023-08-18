use std::error::Error;

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan::api::ApiRequest;

#[derive(Debug, Serialize)]
pub struct GetBucketUsage {
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct GetTotalUsage;

#[derive(Debug, Deserialize)]
pub struct GetUsageLimit;

#[derive(Debug, Deserialize)]
pub struct GetUsageResponse {
    pub size: i64,
}

impl ApiRequest for GetBucketUsage {
    type ErrorType = GetUsageError;
    type ResponseType = GetUsageResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/usage", self.id);
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

impl ApiRequest for GetTotalUsage {
    type ErrorType = GetUsageError;
    type ResponseType = GetUsageResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v1/buckets/usage").unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

impl ApiRequest for GetUsageLimit {
    type ErrorType = GetUsageError;
    type ResponseType = GetUsageResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v1/buckets/usage_limit").unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct GetUsageError {
    #[serde(rename = "error")]
    kind: GetUsageErrorKind,
}

#[derive(Debug, Deserialize)]
enum GetUsageErrorKind {
    Unknown,
}

impl Error for GetUsageError {}

impl std::fmt::Display for GetUsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use GetUsageErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred reading usage",
        };

        f.write_str(msg)
    }
}
