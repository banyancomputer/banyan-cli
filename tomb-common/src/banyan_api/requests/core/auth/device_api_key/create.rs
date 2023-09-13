use crate::banyan_api::requests::ApiRequest;
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use std::error::Error;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct CreateDeviceApiKey {
    pub pem: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDeviceApiKeyResponse {
    pub id: Uuid,
    pub fingerprint: String,
}

impl ApiRequest for CreateDeviceApiKey {
    type ErrorType = CreateDeviceApiKeyError;
    type ResponseType = CreateDeviceApiKeyResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        // Note: This endpoint is only exposed for testing purposes, and should not be used in production.
        let full_url = base_url.join("/api/v1/auth/device_api_key").unwrap();
        client.post(full_url).json(&self)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateDeviceApiKeyError {
    msg: String,
}

impl Error for CreateDeviceApiKeyError {}

impl std::fmt::Display for CreateDeviceApiKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.msg)
    }
}
