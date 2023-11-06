use crate::banyan_api::requests::ApiRequest;
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use std::error::Error;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct DeleteDeviceApiKey {
    pub id: Uuid,
}

impl ApiRequest for DeleteDeviceApiKey {
    type ErrorType = DeleteDeviceApiKeyError;
    type ResponseType = ();

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let id = self.id.to_string();
        let full_url = base_url
            .join(format!("/api/v1/auth/device_api_key/{}", id).as_str())
            .unwrap();
        client.delete(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct DeleteDeviceApiKeyError {
    msg: String,
}

impl Error for DeleteDeviceApiKeyError {}

impl std::fmt::Display for DeleteDeviceApiKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}
