use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan::api::ApiRequest;

#[derive(Debug, Serialize)]
pub struct CreateSnapshot {
    pub bucket_id: Uuid,
    pub metadata_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreateSnapshotResponse {
    pub id: Uuid,
    pub created_at: i64,
}

impl ApiRequest for CreateSnapshot {
    type ResponseType = CreateSnapshotResponse;
    type ErrorType = CreateSnapshotError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/snapshots", self.bucket_id);
        let full_url = base_url.join(&path).unwrap();
        client.post(full_url).json(&self)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct CreateSnapshotError {
    #[serde(rename = "error")]
    kind: CreateSnapshotErrorKind,
}

impl Display for CreateSnapshotError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use CreateSnapshotErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the snapshot",
        };

        f.write_str(msg)
    }
}

impl Error for CreateSnapshotError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum CreateSnapshotErrorKind {
    Unknown,
}
