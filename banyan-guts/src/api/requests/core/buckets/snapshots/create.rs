use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wnfs::libipld::Cid;

use crate::api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct CreateSnapshot {
    pub bucket_id: Uuid,
    pub metadata_id: Uuid,
    pub active_cids: BTreeSet<Cid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSnapshotResponse {
    pub id: Uuid,
}

impl ApiRequest for CreateSnapshot {
    type ResponseType = CreateSnapshotResponse;
    type ErrorType = CreateSnapshotError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!(
            "/api/v1/buckets/{}/metadata/{}/snapshot",
            self.bucket_id, self.metadata_id
        );
        let full_url = base_url.join(&path).unwrap();
        client.post(full_url).json(&self.active_cids)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSnapshotError {
    msg: String,
}

impl Display for CreateSnapshotError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for CreateSnapshotError {}
