use reqwest::{Client, RequestBuilder, Url};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::error::Error;
use crate::banyan::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct CreateDeviceApiKey  {
    pub device_api_key_pem: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDeviceApiKeyResponse {
    pub id: Uuid,
    pub account_id: Uuid,
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
        false
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct CreateDeviceApiKeyError {
    #[serde(rename = "error")]
    kind: CreateDeviceApiKeyErrorKind,
}

#[derive(Debug, Deserialize)]
enum CreateDeviceApiKeyErrorKind {
    Unknown,
}

impl Error for CreateDeviceApiKeyError {}

impl std::fmt::Display for CreateDeviceApiKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CreateDeviceApiKeyErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the account",
        };

        f.write_str(msg)
    }
}
