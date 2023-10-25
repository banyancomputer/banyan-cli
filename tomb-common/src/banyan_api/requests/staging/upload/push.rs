use crate::banyan_api::requests::ApiRequest;
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

#[cfg(not(target_arch = "wasm32"))]
use reqwest::multipart::{Form, Part};
#[cfg(target_arch = "wasm32")]
use std::io::Read;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct PushContent<S>
where
    reqwest::Body: From<S>,
{
    pub host_url: String,
    pub metadata_id: Uuid,
    pub content: S,
    pub content_len: u64,
    pub content_hash: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub struct PushContent<S>
where
    S: Read,
{
    pub host_url: String,
    pub metadata_id: Uuid,
    pub content: S,
    pub content_len: u64,
    pub content_hash: String,
}

#[derive(Debug, Serialize)]
struct PushContentData {
    pub metadata_id: Uuid,
    pub content_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct PushContentResponse {}

#[cfg(not(target_arch = "wasm32"))]
impl<S> ApiRequest for PushContent<S>
where
    reqwest::Body: From<S>,
{
    type ResponseType = PushContentResponse;
    type ErrorType = PushContentError;

    fn build_request(self, _base_url: &Url, client: &Client) -> RequestBuilder {
        let path = "/api/v1/upload".to_string();
        let full_url = Url::parse(&self.host_url).unwrap().join(&path).unwrap();

        // Create our form data
        let pc_req = PushContentData {
            metadata_id: self.metadata_id,
            content_hash: self.content_hash,
        };

        // Attach the form data to the request as json
        let multipart_json_data = serde_json::to_string(&pc_req).unwrap();
        let multipart_json = Part::bytes(multipart_json_data.as_bytes().to_vec())
            .mime_str("application/json")
            .unwrap();

        // Attach the CAR file to the request
        let multipart_car = Part::stream(self.content)
            .mime_str("application/vnd.ipld.car; version=2")
            .unwrap();

        // Combine the two parts into a multipart form
        let multipart_form = Form::new()
            .part("request-data", multipart_json)
            .part("car-upload", multipart_car);

        // post
        client
            .post(full_url)
            .multipart(multipart_form)
            .header(reqwest::header::CONTENT_LENGTH, self.content_len + 546)
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
impl<S> ApiRequest for PushContent<S>
where
    S: Read,
{
    type ResponseType = PushContentResponse;
    type ErrorType = PushContentError;

    fn build_request(mut self, _base_url: &Url, client: &Client) -> RequestBuilder {
        let path = "/api/v1/upload".to_string();
        let full_url = Url::parse(&self.host_url).unwrap().join(&path).unwrap();

        // Create our form data
        let pc_req = PushContentData {
            metadata_id: self.metadata_id,
            content_hash: self.content_hash,
        };

        // Serialize JSON part
        let multipart_json_data = serde_json::to_string(&pc_req).unwrap();

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
        self.content
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
#[non_exhaustive]
pub struct PushContentError {
    #[serde(rename = "msg")]
    message: String,
}

impl Display for PushContentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.message.as_ref())
    }
}

impl Error for PushContentError {}
