use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct DeleteBucket {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct DeleteBucketResponse {}

impl ApiRequest for DeleteBucket {
    type ResponseType = DeleteBucketResponse;
    type ErrorType = DeleteBucketError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}", self.id);
        let url = base_url.join(&path).unwrap();
        client.delete(url).json(&self)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct DeleteBucketError {
    msg: String,
}

impl Display for DeleteBucketError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for DeleteBucketError {}
