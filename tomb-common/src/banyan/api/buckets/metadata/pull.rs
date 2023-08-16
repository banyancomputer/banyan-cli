use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan::api::StreamableApiRequest;

#[derive(Debug, Serialize)]
pub struct PullBucketMetadata
{
    pub id: Uuid,
    pub bucket_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct PullBucketMetadataResponse(pub(crate) Vec<u8>);

impl StreamableApiRequest for PullBucketMetadata {
    type ErrorType = PullBucketMetadataError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/metadata/{}/pull", self.bucket_id, self.id);
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct PullBucketMetadataError {
    #[serde(rename = "error")]
    kind: PullBucketMetadataErrorKind,
}

impl Display for PullBucketMetadataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use PullBucketMetadataErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the bucket",
        };

        f.write_str(msg)
    }
}

impl Error for PullBucketMetadataError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum PullBucketMetadataErrorKind {
    Unknown,
}
