use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::Deserialize;

use crate::api::{models::bucket::Bucket, requests::ApiRequest};

pub struct UpdateBucket(pub Bucket);

#[derive(Deserialize)]
pub struct UpdateBucketResponse;

impl ApiRequest for UpdateBucket {
    type ResponseType = UpdateBucketResponse;
    type ErrorType = UpdateBucketError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let url = base_url
            .join(&format!("/api/v1/buckets/{}", self.0.id))
            .unwrap();
        client.put(url).json(&self.0)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct UpdateBucketError {
    msg: String,
}

impl Display for UpdateBucketError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for UpdateBucketError {}
