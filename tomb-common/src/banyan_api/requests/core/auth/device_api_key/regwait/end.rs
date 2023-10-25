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

#[cfg(test)]
#[cfg(feature = "fake")]
pub mod test {
    use super::*;
    use crate::banyan_api::{
        models::account::test::authenticated_client,
        requests::core::auth::device_api_key::regwait::start::StartRegwait,
        utils::generate_api_key,
    };
    use std::sync::Arc;
    use tomb_crypt::prelude::PrivateKey;

    #[tokio::test]
    #[ignore]
    async fn regwait_success() {
        let mut client = authenticated_client().await;
        let mut other_client = client.clone();
        let (api_key, _pem) = generate_api_key().await;
        let fingerprint_arc_bytes = api_key
            .fingerprint()
            .await
            .expect("Failed to get fingerprint");
        let fingerprint_bytes =
            Arc::into_inner(fingerprint_arc_bytes).expect("Failed to get fingerprint bytes");
        let fingerprint = fingerprint_bytes
            .iter()
            .fold(String::new(), |chain, byte| format!("{chain}{byte:02x}"));
        let fingerprint_clone = fingerprint.clone();

        let end_handle = tokio::spawn(async move {
            std::thread::sleep(std::time::Duration::from_secs(1));
            other_client
                .call_no_content(EndRegwait {
                    fingerprint: fingerprint_clone,
                })
                .await
                .unwrap();
        });
        client
            .call_no_content(StartRegwait { fingerprint })
            .await
            .unwrap();
        end_handle.await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn regwait_fail() {
        let mut client = authenticated_client().await;
        let mut other_client = client.clone();
        let end_handle = tokio::spawn(async move {
            std::thread::sleep(std::time::Duration::from_secs(1));
            other_client
                .call_no_content(EndRegwait {
                    fingerprint: "other_random_nonsense_string".to_string(),
                })
                .await
                .expect_err("Expected an error");
        });
        client
            .call_no_content(StartRegwait {
                fingerprint: "random_nonsense_string".to_string(),
            })
            .await
            .expect_err("Expected an error");
        end_handle.await.unwrap();
    }
}
