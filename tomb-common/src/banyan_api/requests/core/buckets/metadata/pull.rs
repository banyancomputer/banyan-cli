use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan_api::requests::StreamableApiRequest;

#[derive(Debug, Serialize)]
pub struct PullMetadata {
    pub id: Uuid,
    pub bucket_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct PullMetadataResponse(pub(crate) Vec<u8>);

impl StreamableApiRequest for PullMetadata {
    type ErrorType = PullMetadataError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!(
            "/api/v1/buckets/{}/metadata/{}/pull",
            self.bucket_id, self.id
        );
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct PullMetadataError {
    msg: String,
}

impl Display for PullMetadataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for PullMetadataError {}
