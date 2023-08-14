use super::{credentials::Credentials, error::ClientError, requests::ApiRequest};
use anyhow::Result;
use reqwest::{Client as ReqwestClient, Url, header::{HeaderMap, HeaderValue}};
use tomb_crypt::prelude::*;

/// The audience for the API token
const AUDIENCE: &str = "banyan-platform";

#[derive(Debug)]
/// Client for interacting with our API
pub struct Client {
    /// Base URL
    pub remote: Url,
    /// Bearer auth
    pub claims: Option<ApiToken>,
    /// Credentials for signing
    pub signing_key: Option<EcSignatureKey>,
    /// The current bearer token
    pub bearer_token: Option<String>,

    reqwest_client: ReqwestClient
}

impl Client {
    /// Create a new Client at a remote endpoint
    /// # Arguments
    /// * `remote` - The base URL for the API
    /// # Returns
    /// * `Self` - The client
    pub fn new(remote: &str) -> Result<Self> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            "Content-Type",
            HeaderValue::from_static("application/json"),
        );
        let reqwest_client = ReqwestClient::builder()
            .default_headers(default_headers)
            .build()
            .unwrap();

        Ok(Self {
            remote: Url::parse(remote)?,
            claims: None,
            signing_key: None,
            bearer_token: None,
            reqwest_client
        })
    }

    /// Set the credentials for signing
    /// # Arguments
    /// * `credentials` - The credentials to use for signing
    pub fn with_credentials(&mut self, credentials: Credentials) {
        self.bearer_token = None;
        self.claims = Some(ApiToken::new(
            AUDIENCE.to_string(),
            credentials.account_id.to_string(),
        ));
        self.signing_key = Some(credentials.signing_key);
    }

    /// Set the bearer token directly
    /// # Arguments
    /// * `bearer_token` - The bearer token to use
    pub fn with_bearer_token(&mut self, bearer_token: String) {
        self.claims = None;
        self.signing_key = None;
        self.bearer_token = Some(bearer_token);
    }

    /// Return a bearer token based on the current credentials
    /// # Returns
    /// * `Option<String>` - The bearer token
    /// # Errors
    /// * `ClientError` - If there is an error generating the token.
    ///    If the bearer token can not be encoded, or if the signing key is not available.
    pub async fn bearer_token(&mut self) -> Result<String, ClientError> {
        match &self.claims {
            Some(claims) => {
                let is_expired = claims.is_expired().map_err(ClientError::crypto_error)?;
                // If we already have a bearer token and the claims are still valid
                // return the current bearer token
                if !is_expired && self.bearer_token.is_some() {
                    return Ok(self.bearer_token.clone().unwrap());
                } else if is_expired {
                    claims.refresh().map_err(ClientError::crypto_error)?;
                }
                match &self.signing_key {
                    Some(signing_key) => {
                        self.bearer_token = Some(
                            claims
                                .encode_to(signing_key)
                                .await
                                .map_err(ClientError::crypto_error)?,
                        );
                        Ok(self.bearer_token.clone().unwrap())
                    }
                    _ => Err(ClientError::auth_unavailable()),
                }
            }
            // No claims, so no bearer token
            _ => match &self.bearer_token {
                Some(bearer_token) => Ok(bearer_token.clone()),
                _ => Err(ClientError::auth_unavailable()),
            },
        }
    }

    pub async fn call<T: ApiRequest>(
        &mut self,
        request: T,
    ) -> Result<T::ResponseType, ClientError> {
        let add_authentication = request.requires_authentication();
        let mut request_builder = request.build_request(&self.remote, &self.reqwest_client);
        if add_authentication {
            let bearer_token = self.bearer_token().await?;
            request_builder = request_builder.bearer_auth(bearer_token);
        }

        let response = request_builder
            .send()
            .await
            .map_err(ClientError::http_error)?;

        if response.status().is_success() {
            response
                .json::<T::ResponseType>()
                .await
                .map_err(ClientError::bad_format)
        } else {
            let err = response
                .json::<T::ErrorType>()
                .await
                .map_err(ClientError::bad_format)?;

            let err = Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>;
            Err(ClientError::from(err))
        }
    }
}