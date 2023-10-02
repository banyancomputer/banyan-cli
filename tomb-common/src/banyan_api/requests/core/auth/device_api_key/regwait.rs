use crate::banyan_api::requests::ApiRequest;
use serde::{Deserialize, Serialize};
use std::error::Error;
use uuid::Uuid;

#[derive(Debug, Serialize, Default)]
pub struct Regwait {
    pub nonce: Uuid,
}

impl Regwait {
    pub fn new() -> Self {
        Self {
            nonce: uuid::Uuid::new_v4(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RegwaitResponse {
    pub account_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct RegwaitError {
    msg: String,
}

impl std::fmt::Display for RegwaitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for RegwaitError {}

impl ApiRequest for Regwait {
    type ResponseType = RegwaitResponse;
    type ErrorType = RegwaitError;

    fn build_request(
        self,
        base_url: &reqwest::Url,
        client: &reqwest::Client,
    ) -> reqwest::RequestBuilder {
        // Create the full url
        let full_url = base_url
            .join(&format!(
                "/api/v1/auth/device_api_key/start_regwait/{}",
                self.nonce
            ))
            .unwrap();
        // Run a get request
        client.get(full_url).json(&self)
    }

    fn requires_authentication(&self) -> bool {
        // No auth required
        false
    }
}
