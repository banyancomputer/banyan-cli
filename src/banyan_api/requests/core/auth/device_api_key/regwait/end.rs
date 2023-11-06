use crate::banyan_api::requests::ApiRequest;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Serialize, Default)]
pub struct EndRegwait {
    pub fingerprint: String,
}

#[derive(Debug, Deserialize)]
pub struct EndRegwaitResponse;

#[derive(Debug, Deserialize)]
pub struct EndRegwaitError {
    msg: String,
}

impl std::fmt::Display for EndRegwaitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for EndRegwaitError {}

impl ApiRequest for EndRegwait {
    type ResponseType = EndRegwaitResponse;
    type ErrorType = EndRegwaitError;

    fn build_request(
        self,
        base_url: &reqwest::Url,
        client: &reqwest::Client,
    ) -> reqwest::RequestBuilder {
        // Create the full url
        let full_url = base_url
            .join(&format!(
                "/api/v1/auth/device_api_key/end_regwait/{}",
                self.fingerprint
            ))
            .unwrap();
        // Run a get request
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        // Auth is required - this will only be called from WASM
        true
    }
}
