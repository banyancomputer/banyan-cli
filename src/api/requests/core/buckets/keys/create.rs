use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct CreateBucketKey {
    pub bucket_id: Uuid,
    pub pem: String,
}

#[derive(Debug, Serialize)]
struct CreateBucketKeyLess {
    pub pem: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateBucketKeyResponse {
    pub id: Uuid,
    pub approved: bool,
    pub fingerprint: String,
}

impl ApiRequest for CreateBucketKey {
    type ResponseType = CreateBucketKeyResponse;
    type ErrorType = CreateBucketKeyError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/keys", self.bucket_id);
        let full_url = base_url.join(&path).unwrap();
        client
            .post(full_url)
            .json(&CreateBucketKeyLess { pem: self.pem })
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateBucketKeyError {
    msg: String,
}

impl Display for CreateBucketKeyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for CreateBucketKeyError {}
