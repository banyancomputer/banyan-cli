use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan_api::requests::ApiRequest;

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
        let path = format!(
            "/api/v1/buckets/{}/snapshots/{}/restore",
            self.bucket_id, self.snapshot_id
        );
        let full_url = base_url.join(&path).unwrap();
        client.put(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct RestoreSnapshotError {
    msg: String,
}

impl Display for RestoreSnapshotError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for RestoreSnapshotError {}
