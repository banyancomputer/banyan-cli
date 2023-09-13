use crate::banyan_api::requests::ApiRequest;
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use std::error::Error;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ReadDeviceApiKey {
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ReadAllDeviceApiKeys;

#[derive(Debug, Serialize, Deserialize)]
pub struct ReadDeviceApiKeyResponse {
    pub id: Uuid,
    pub pem: String,
    pub fingerprint: String,
}

#[derive(Debug, Deserialize)]
pub struct ReadAllDeviceApiKeysResponse(pub(crate) Vec<ReadDeviceApiKeyResponse>);

impl ApiRequest for ReadDeviceApiKey {
    type ErrorType = ReadDeviceApiKeyError;
    type ResponseType = ReadDeviceApiKeyResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let device_api_key_id = self.id.to_string();
        let full_url = base_url
            .join(format!("/api/v1/auth/device_api_key/{}", device_api_key_id).as_str())
            .unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

impl ApiRequest for ReadAllDeviceApiKeys {
    type ErrorType = ReadDeviceApiKeyError;
    type ResponseType = ReadAllDeviceApiKeysResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v1/auth/device_api_key").unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct ReadDeviceApiKeyError {
    msg: String,
}

impl std::fmt::Display for ReadDeviceApiKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for ReadDeviceApiKeyError {}
