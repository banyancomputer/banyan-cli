use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan_api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct ReadAllSnapshots {
    pub bucket_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ReadSnapshotResponse {
    pub id: Uuid,
    pub metadata_id: Uuid,
    pub created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct ReadAllSnapshotResponse(pub(crate) Vec<ReadSnapshotResponse>);

impl ApiRequest for ReadAllSnapshots {
    type ResponseType = ReadAllSnapshotResponse;
    type ErrorType = ReadSnapshotError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/snapshots", self.bucket_id);
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ReadSnapshotError {
    #[serde(rename = "error")]
    kind: ReadSnapshotErrorKind,
}

impl Display for ReadSnapshotError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ReadSnapshotErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the bucket",
        };

        f.write_str(msg)
    }
}

impl Error for ReadSnapshotError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum ReadSnapshotErrorKind {
    Unknown,
}
