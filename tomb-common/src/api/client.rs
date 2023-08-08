use anyhow::Result;
use reqwest::Url;
use crate::api::request::Respondable;
use super::{request::Requestable, error::ClientError, token::Token, credentials::Credentials};

pub struct Client {
    remote: Url,
    token: Option<Token>,
    credentials: Option<Credentials>
}

impl Client {
    pub fn new(remote: &str) -> Result<Self> {
        Ok(Self {
            remote: Url::parse(remote)?,
            token: None,
            credentials: None
        })
    }

    pub async fn send<R: Requestable>(&self, request: R) -> Result<R::ResponseType, ClientError> {
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
            .build().unwrap();

        // Create the RequestBuilder
        let mut builder = client
            .request(request.method(), full_url)
            .json(&request);

        // If the request requires authentication
        if request.authed() {
            // If we have a Token and Credentials present
            if let Some(token) = &self.token && let Some(credentials) = &self.credentials {
                // Sign a new token
                let signed_token = token.sign(&credentials.fingerprint, &credentials.signing_key);
                // Apply bearer Authentication
                builder = builder.bearer_auth(signed_token);
            } else {
                // Auth was not available but was required
                return Err(ClientError::auth_unavailable());
            }
        }

        // Send and await the response
        let response = builder.send().await.map_err(ClientError::http_error)?;

        let x = R::ResponseType::process(request, response).await;


        // If we succeeded
        if response.status().is_success() {
            let x: R::ResponseType = R::ResponseType::process(request, response).await.unwrap();
            Ok(x)
            // response
            //     .json::<R::ResponseType>()
            //     .await
            //     .map_err(ClientError::bad_format)
        } else {
            let err = response
                .json::<R::ErrorType>()
                .await
                .map_err(ClientError::bad_format)?;

            let err = Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>;
            Err(ClientError::from(err))
        }
    }
}