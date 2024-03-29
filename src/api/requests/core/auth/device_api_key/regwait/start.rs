use crate::api::requests::ApiRequest;
use serde::{Deserialize, Serialize};
use std::error::Error;
use uuid::Uuid;

#[derive(Debug, Serialize, Default)]
pub struct StartRegwait {
    pub fingerprint: String,
}

#[derive(Debug, Deserialize)]
pub struct StartRegwaitResponse {
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct StartRegwaitError {
    msg: String,
}

impl std::fmt::Display for StartRegwaitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for StartRegwaitError {}

impl ApiRequest for StartRegwait {
    type ResponseType = StartRegwaitResponse;
    type ErrorType = StartRegwaitError;

    fn build_request(
        self,
        base_url: &reqwest::Url,
        client: &reqwest::Client,
    ) -> reqwest::RequestBuilder {
        // Create the full url
        let full_url = base_url
            .join(&format!(
                "/api/v1/auth/device_api_key/start_regwait/{}",
                self.fingerprint
            ))
            .unwrap();
        // Run a get request
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        // No auth required
        false
    }
}
