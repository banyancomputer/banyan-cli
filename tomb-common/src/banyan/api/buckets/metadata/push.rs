use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan::models::bucket_metadata::BucketMetadataState;
use crate::banyan::api::ApiRequest;

#[derive(Debug, Serialize)]
pub struct PushBucketMetadata<S> 
where
    reqwest::Body: From<S>,
{
    pub bucket_id: Uuid,

    pub data_size: usize,
    pub metadata_cid: String,
    pub root_cid: String,

    pub metadata_stream: S,
}

#[derive(Debug, Serialize)]
pub struct PushBucketMetadataData {
    pub data_size: usize,
    pub metadata_cid: String,
    pub root_cid: String,
}

#[derive(Debug, Deserialize)]
pub struct PushBucketMetadataResponse {
    pub id: Uuid,
    pub state: BucketMetadataState,
    pub storage_host: String,
    pub storage_authorization: String,
}

impl<S> ApiRequest for PushBucketMetadata<S> 
where
    reqwest::Body: From<S>,
{
    type ResponseType = PushBucketMetadataResponse;
    type ErrorType = PushBucketMetadataError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/metadata", self.bucket_id);
        let full_url = base_url.join(&path).unwrap();

        // Create our form data
        let pbm_req = PushBucketMetadataData {
            data_size: self.data_size,
            metadata_cid: self.metadata_cid,
            root_cid: self.root_cid,
        };

        // Attach the form data to the request as json
        let multipart_json_data = serde_json::to_string(&pbm_req).unwrap();
        let multipart_json =
            reqwest::multipart::Part::bytes(multipart_json_data.as_bytes().to_vec())
                .mime_str("application/json")
                .unwrap();
        // Attach the CAR file to the request
        let multipart_car = reqwest::multipart::Part::stream(self.metadata_stream)
            .mime_str("application/vnd.ipld.car; version=2")
            .unwrap();
        // Combine the two parts into a multipart form
        let multipart_form = reqwest::multipart::Form::new()
            .part("request-data", multipart_json)
            .part("car-upload", multipart_car);
        // post
        client.post(full_url).multipart(multipart_form)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct PushBucketMetadataError {
    #[serde(rename = "error")]
    kind: PushBucketMetadataErrorKind,
}

impl Display for PushBucketMetadataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use PushBucketMetadataErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the bucket",
        };

        f.write_str(msg)
    }
}

impl Error for PushBucketMetadataError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum PushBucketMetadataErrorKind {
    Unknown,
}
