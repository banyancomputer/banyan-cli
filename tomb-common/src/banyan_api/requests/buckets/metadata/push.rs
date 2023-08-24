use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::banyan_api::models::metadata::MetadataState;
use crate::banyan_api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct PushMetadata<S>
where
    reqwest::Body: From<S>,
{
    pub bucket_id: Uuid,

    pub expected_data_size: usize,
    pub metadata_cid: String,
    pub root_cid: String,

    pub metadata_stream: S,
}

#[derive(Debug, Serialize)]
struct PushMetadataData {
    pub expected_data_size: usize,
    pub metadata_cid: String,
    pub root_cid: String,
}

#[derive(Debug, Deserialize)]
pub struct PushMetadataResponse {
    pub id: Uuid,
    pub state: MetadataState,
    pub storage_host: Option<String>,
    pub storage_authorization: Option<String>,
}

impl<S> ApiRequest for PushMetadata<S>
where
    reqwest::Body: From<S>,
{
    type ResponseType = PushMetadataResponse;
    type ErrorType = PushMetadataError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/metadata", self.bucket_id);
        let full_url = base_url.join(&path).unwrap();

        // Create our form data
        let pbm_req = PushMetadataData {
            expected_data_size: self.expected_data_size,
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
pub struct PushMetadataError {
    #[serde(rename = "error")]
    kind: PushMetadataErrorKind,
}

impl Display for PushMetadataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use PushMetadataErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the bucket",
        };

        f.write_str(msg)
    }
}

impl Error for PushMetadataError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum PushMetadataErrorKind {
    Unknown,
}
