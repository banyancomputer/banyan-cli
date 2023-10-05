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
        println!("base_url: {:?}", base_url);
        // Create the full url
        let full_url = base_url
            .join(&format!(
                "/api/v1/auth/device_api_key/end_regwait/{}",
                self.fingerprint
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

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::banyan_api::models::account::test::unauthenticated_client;

    #[tokio::test]
    async fn regwait_fail() {
        let mut client = unauthenticated_client().await;
        // Try to end the regwait on a nonexistent fingerprint
        let result = client
            .call(EndRegwait {
                fingerprint: "random_nonsense_string".to_string(),
            })
            .await;

        println!("result: {:?}", result);
    }
}
