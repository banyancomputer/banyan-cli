use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan::api::ApiRequest;

#[derive(Debug, Serialize)]
pub struct RestoreSnapshot {
    pub bucket_id: Uuid,
    pub snapshot_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RestoreSnapshotResponse {
    pub metadata_id: Uuid,
}

impl ApiRequest for RestoreSnapshot {
    type ResponseType = RestoreSnapshotResponse;
    type ErrorType = RestoreSnapshotError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/snapshots/{}/restore", self.bucket_id, self.snapshot_id);
        let full_url = base_url.join(&path).unwrap();
        client.put(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct RestoreSnapshotError {
    #[serde(rename = "error")]
    kind: RestoreSnapshotErrorKind,
}

impl Display for RestoreSnapshotError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use RestoreSnapshotErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the bucket",
        };

        f.write_str(msg)
    }
}

impl Error for RestoreSnapshotError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum RestoreSnapshotErrorKind {
    Unknown,
}
