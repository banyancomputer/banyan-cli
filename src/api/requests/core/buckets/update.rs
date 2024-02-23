use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::Deserialize;

use crate::api::requests::ApiRequest;

pub struct UpdateBucket {
    pub bucket_id: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct UpdateBucketResponse;

impl ApiRequest for UpdateBucket {
    type ResponseType = UpdateBucketResponse;
    type ErrorType = UpdateBucketError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let url = base_url
            .join(&format!("/api/v1/buckets/{}", self.bucket_id))
            .unwrap();

        client.put(url).json(&self.name)
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
