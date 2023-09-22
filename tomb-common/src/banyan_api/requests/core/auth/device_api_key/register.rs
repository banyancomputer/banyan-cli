use crate::banyan_api::requests::ApiRequest;
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use std::error::Error;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct RegisterDeviceApiKey {
    pub pem: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterDeviceApiKeyResponse {
    #[serde(rename="accountId")]
    pub account_id: Uuid,
    pub fingerprint: String,
    pub pem: String,
}

impl ApiRequest for RegisterDeviceApiKey {
    type ErrorType = RegisterDeviceApiKeyError;
    type ResponseType = RegisterDeviceApiKeyResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        // Strip the public key of its new lines
        let mut stripped_public_key = self.pem.replace("\n", "");
        // Strip the public key of its prefix and suffix
        stripped_public_key = stripped_public_key
            .strip_prefix("-----BEGIN PUBLIC KEY-----")
            .unwrap()
            .strip_suffix("-----END PUBLIC KEY-----")
            .unwrap()
            .to_string();

        // Represent the weird b64 characters with ones that are url-valid
        let encoded_public_key = stripped_public_key.replace("+", "-").replace(r#"/"#, "_").replace("=", ".").to_string();
        
        // Note: This endpoint is only exposed for testing purposes, and should not be used in production.
        let full_url = base_url.join(&format!("/api/auth/device/register?spki={}", encoded_public_key)).unwrap();

        // open::that(full_url.to_string()).expect("msg");

        println!("full url: {}", full_url);
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        false
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterDeviceApiKeyError {
    msg: String,
}

impl Error for RegisterDeviceApiKeyError {}

impl std::fmt::Display for RegisterDeviceApiKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}
