use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct DeleteBucketKey {
    pub bucket_id: Uuid,
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct DeleteBucketKeyResponse {
    pub id: Uuid,
    pub approved: bool,
}

impl ApiRequest for DeleteBucketKey {
    type ResponseType = DeleteBucketKeyResponse;
    type ErrorType = DeleteBucketKeyError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/keys/{}", self.bucket_id, self.id);
        let full_url = base_url.join(&path).unwrap();
        client.delete(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct DeleteBucketKeyError {
    msg: String,
}

impl Display for DeleteBucketKeyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for DeleteBucketKeyError {}
