use std::error::Error;

use super::{credentials::Credentials, error::ClientError, request::Requestable, token::Token};
use crate::api::request::Respondable;
use anyhow::Result;
use reqwest::Url;
use serde::de::DeserializeOwned;

pub struct Client {
    pub remote: Url,
    pub bearer_token: Option<String>,
    pub credentials: Option<Credentials>,
}

impl Client {
    pub fn new(remote: &str) -> Result<Self> {
        Ok(Self {
            remote: Url::parse(remote)?,
            bearer_token: None,
            credentials: None,
        })
    }

    pub async fn send<
        R: Requestable,
        ResponseType: DeserializeOwned,
        ErrorType: DeserializeOwned + Error + Send + Sync + 'static,
    >(
        &self,
        request: R,
    ) -> Result<ResponseType, ClientError> {
        // Determine the full URL to send the request to
        // This should never fail
        let full_url = self.remote.join(&request.endpoint()).unwrap();

        // Default header
        let mut default_headers = reqwest::header::HeaderMap::new();
        default_headers.insert(
            "Content-Type",
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        // Create the Client
        let client = reqwest::Client::builder()
            .default_headers(default_headers)
            .user_agent("banyan-api-client/0.1.0")
            .build()
            .unwrap();

        // Create the RequestBuilder
        let mut builder = client.request(request.method(), full_url).json(&request);

        // Apply bearer Authentication
        if let Some(bearer_token) = &self.bearer_token {
            builder = builder.bearer_auth(bearer_token);
        }

        // If the request requires authentication
        if request.authed() && (self.bearer_token.is_none() || self.credentials.is_none()) {
            // Auth was not available but was required
            return Err(ClientError::auth_unavailable());
        }

        // Send and await the response
        let response = builder.send().await.map_err(ClientError::http_error)?;
        // If we succeeded
        if response.status().is_success() {
            // let r2 = response;
            let bytes = response.bytes().await.unwrap().to_vec();
            println!("response as str: {}", String::from_utf8(bytes.clone()).unwrap());

            Ok(serde_json::from_slice(&bytes).unwrap())
        } else {
            // let r2 = response;
            let bytes = response.bytes().await.unwrap().to_vec();
            println!("error as str: {}", String::from_utf8(bytes).unwrap());
            Err(ClientError::auth_unavailable())
        }
    }
}
