use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct ReadAllSnapshots {
    pub bucket_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ReadSingleSnapshot {
    pub bucket_id: Uuid,
    pub snapshot_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ReadSnapshotResponse {
    pub id: Uuid,
    pub metadata_id: Uuid,
    pub size: Option<u64>,
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

impl ApiRequest for ReadSingleSnapshot {
    type ResponseType = ReadSnapshotResponse;
    type ErrorType = ReadSnapshotError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!(
            "/api/v1/buckets/{}/snapshots/{}",
            self.bucket_id, self.snapshot_id
        );
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct ReadSnapshotError {
    msg: String,
}

impl Display for ReadSnapshotError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for ReadSnapshotError {}

#[cfg(test)]
use crate::prelude::api::models::snapshot::Snapshot;
#[cfg(test)]
impl ReadSnapshotResponse {
    #[allow(dead_code)]
    pub(crate) fn to_snapshot(&self, bucket_id: Uuid) -> Snapshot {
        Snapshot {
            id: self.id,
            bucket_id,
            metadata_id: self.metadata_id,
            size: self.size.unwrap_or(0),
            created_at: self.created_at,
        }
    }
}
