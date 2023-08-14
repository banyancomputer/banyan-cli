use reqwest::{Client, RequestBuilder, Url};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use std::error::Error;
use crate::banyan::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct CreateAccount {
    pub device_api_key_pem: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAccountResponse {
    pub id: Uuid,
}

impl ApiRequest for CreateAccount {
    type ErrorType = CreateAccountError;
    type ResponseType = CreateAccountResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        // Note: This endpoint is only exposed for testing purposes, and should not be used in production.
        let full_url = base_url.join("/api/v1/auth/fake_account").unwrap();
        client.post(full_url).json(&self)
    }

    fn requires_authentication(&self) -> bool {
        false
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct CreateAccountError {
    #[serde(rename = "error")]
    kind: CreateAccountErrorKind,
}

#[derive(Debug, Deserialize)]
enum CreateAccountErrorKind {
    Unknown,
}

impl Error for CreateAccountError {}

impl std::fmt::Display for CreateAccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use CreateAccountErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the account",
        };

        f.write_str(msg)
    }
}
