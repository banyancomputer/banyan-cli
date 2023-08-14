use reqwest::{Client, RequestBuilder, Url};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::error::Error;
use crate::banyan::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct DeleteDeviceApiKey  {
    pub id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteDeviceApiKeyResponse {
    pub id: Uuid,
    pub account_id: Uuid,
    pub fingerprint: String,
}

impl ApiRequest for DeleteDeviceApiKey {
    type ErrorType = DeleteDeviceApiKeyError;
    type ResponseType = DeleteDeviceApiKeyResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let id = self.id.to_string();
        let full_url = base_url.join(format!("/api/v1/auth/device_api_key/{}", id).as_str()).unwrap();
        client.delete(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct DeleteDeviceApiKeyError {
    #[serde(rename = "error")]
    kind: DeleteDeviceApiKeyErrorKind,
}

#[derive(Debug, Deserialize)]
enum DeleteDeviceApiKeyErrorKind {
    Unknown,
}

impl Error for DeleteDeviceApiKeyError {}

impl std::fmt::Display for DeleteDeviceApiKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use DeleteDeviceApiKeyErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred deleting the device api key",
        };

        f.write_str(msg)
    }
}
