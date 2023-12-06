use crate::api::{models::metadata::MetadataState, requests::ApiRequest};
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

#[cfg(not(target_arch = "wasm32"))]
use reqwest::Body;
#[cfg(target_arch = "wasm32")]
use std::io::{Cursor, Read};

#[cfg(not(target_arch = "wasm32"))]
pub type MetadataStreamType = Body;
#[cfg(target_arch = "wasm32")]
pub type MetadataStreamType = Cursor<Vec<u8>>;

#[derive(Debug)]
pub struct PushMetadata {
    pub bucket_id: Uuid,

    pub expected_data_size: u64,
    pub root_cid: String,
    pub metadata_cid: String,
    pub previous_metadata_cid: Option<String>,
    pub valid_keys: Vec<String>,
    pub deleted_block_cids: BTreeSet<String>,

    pub metadata_stream: MetadataStreamType,
}

#[derive(Debug, Serialize)]
struct PushMetadataData {
    pub expected_data_size: u64,
    pub root_cid: String,
    pub metadata_cid: String,
    pub previous_metadata_cid: Option<String>,
    pub valid_keys: Vec<String>,
    pub deleted_block_cids: BTreeSet<String>,
}

#[derive(Debug, Deserialize)]
pub struct PushMetadataResponse {
    pub id: Uuid,
    pub state: MetadataState,
    pub storage_host: Option<String>,
    pub storage_authorization: Option<String>,
}

#[cfg(not(target_arch = "wasm32"))]
impl ApiRequest for PushMetadata {
    type ResponseType = PushMetadataResponse;
    type ErrorType = PushMetadataError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/metadata", self.bucket_id);
        let full_url = base_url.join(&path).unwrap();

        // Create our form data
        let pbm_req = PushMetadataData {
            expected_data_size: self.expected_data_size,
            root_cid: self.root_cid,
            metadata_cid: self.metadata_cid,
            previous_metadata_cid: self.previous_metadata_cid,
            valid_keys: self.valid_keys,
            deleted_block_cids: self.deleted_block_cids,
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

#[cfg(target_arch = "wasm32")]
fn generate_boundary() -> String {
    use rand::{distributions::Alphanumeric, Rng};
    let random_string: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30) // Adjust the length as needed
        .map(char::from)
        .collect();

    format!("------------------------{}", random_string)
}

#[cfg(target_arch = "wasm32")]
impl ApiRequest for PushMetadata {
    type ResponseType = PushMetadataResponse;
    type ErrorType = PushMetadataError;

    fn build_request(mut self, base_url: &Url, client: &Client) -> RequestBuilder {
        let path = format!("/api/v1/buckets/{}/metadata", self.bucket_id);
        let full_url = base_url.join(&path).unwrap();

        // Create our form data
        let pbm_req = PushMetadataData {
            expected_data_size: self.expected_data_size,
            root_cid: self.root_cid,
            metadata_cid: self.metadata_cid,
            valid_keys: self.valid_keys,
            deleted_block_cids: self.deleted_block_cids,
        };

        // Serialize JSON part
        let multipart_json_data = serde_json::to_string(&pbm_req).unwrap();

        // Generate boundary
        let boundary = generate_boundary();

        // Construct multipart body manually
        let mut multipart_body = Vec::new();

        multipart_body.extend(format!("--{}\r\n", boundary).as_bytes());
        multipart_body.extend(b"Content-Disposition: form-data; name=\"request-data\"\r\n");
        multipart_body.extend(b"Content-Type: application/json\r\n\r\n");
        multipart_body.extend(multipart_json_data.as_bytes());
        multipart_body.extend(b"\r\n");

        multipart_body.extend(format!("--{}\r\n", boundary).as_bytes());
        multipart_body.extend(b"Content-Disposition: form-data; name=\"car-upload\"\r\n");
        multipart_body.extend(b"Content-Type: application/vnd.ipld.car; version=2\r\n\r\n");

        // If S implements the Read trait, this will work:
        let mut buffer = Vec::new();
        self.metadata_stream
            .read_to_end(&mut buffer)
            .expect("Failed to read metadata stream to bytes");
        multipart_body.extend(&buffer);

        multipart_body.extend(b"\r\n");

        multipart_body.extend(format!("--{}--\r\n", boundary).as_bytes()); // Closing boundary

        // Set headers
        let content_type = format!("multipart/form-data; boundary={}", boundary);

        client
            .post(full_url)
            .body(multipart_body)
            .header("Content-Type", content_type)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct PushMetadataError {
    msg: String,
}

impl Display for PushMetadataError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for PushMetadataError {}
