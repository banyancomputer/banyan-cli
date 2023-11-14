use super::{
    error::ApiError,
    requests::{ApiRequest, StreamableApiRequest},
};
use bytes::Bytes;
use futures_core::stream::Stream;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client as ReqwestClient, Url,
};
use std::{fmt::Debug, string::ParseError};
use tomb_crypt::prelude::{ApiToken, EcSignatureKey};
use uuid::Uuid;

#[derive(Clone)]
/// Credentials in order to sign and verify messages for a Banyan account
pub struct Credentials {
    /// The unique account id (used as a JWT subject)
    pub user_id: Uuid,
    /// The signing key (used to sign JWTs)
    pub signing_key: EcSignatureKey,
}

impl Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Get the pem string for the signing key
        f.debug_struct("Credentials")
            .field("user_id", &self.user_id)
            .finish()
    }
}

impl Credentials {
    /// Create a new set of credentials
    pub fn new(user_id: String, signing_key: EcSignatureKey) -> Result<Self, uuid::Error> {
        let user_id = Uuid::parse_str(&user_id)?;
        Ok(Self {
            user_id,
            signing_key,
        })
    }
}

/// The audience for the API token
const AUDIENCE: &str = "banyan-platform";

#[derive(Debug, Clone)]
/// Client for interacting with our API
pub struct Client {
    /// Base URL for interacting with core service
    pub remote_core: Url,
    /// Base URL for pulling data
    pub remote_data: Url,
    /// Bearer auth
    pub claims: Option<ApiToken>,
    /// Credentials for signing
    pub signing_key: Option<EcSignatureKey>,
    /// The current bearer token
    pub bearer_token: Option<String>,
    /// The reqwest client
    reqwest_client: ReqwestClient,
}

impl Client {
    /// Create a new Client at a remote endpoint
    /// # Arguments
    /// * `remote` - The base URL for the API
    /// # Returns
    /// * `Self` - The client
    pub fn new(remote_core: &str, remote_data: &str) -> Result<Self, ApiError> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        let reqwest_client = ReqwestClient::builder()
            .default_headers(default_headers)
            .build()
            .map_err(ApiError::reqwest_general)?;

        Ok(Self {
            remote_core: Url::parse(remote_core)?,
            remote_data: Url::parse(remote_data)?,
            claims: None,
            signing_key: None,
            bearer_token: None,
            reqwest_client,
        })
    }

    /// Set a new remote endpoint
    /// # Arguments
    /// * `remote` - The base URL for the API
    /// # Returns
    /// * `Self` - The client
    pub fn with_remote(&mut self, remote: &str) -> Result<(), ApiError> {
        self.remote_core = Url::parse(remote)?;
        Ok(())
    }

    /// Set the credentials for signing
    /// # Arguments
    /// * `credentials` - The credentials to use for signing
    pub fn with_credentials(&mut self, credentials: Credentials) {
        self.bearer_token = None;
        self.claims = Some(ApiToken::new(
            AUDIENCE.to_string(),
            credentials.user_id.to_string(),
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

    /// Log out of the account
    pub fn logout(&mut self) {
        self.claims = None;
        self.signing_key = None;
        self.bearer_token = None;
    }

    /// Return a bearer token based on the current credentials
    /// # Returns
    /// * `Option<String>` - The bearer token
    /// # Errors
    /// * `ClientError` - If there is an error generating the token.
    ///    If the bearer token can not be encoded, or if the signing key is not available.
    pub async fn bearer_token(&mut self) -> Result<String, ApiError> {
        match &self.claims {
            Some(claims) => {
                let is_expired = claims.is_expired()?;
                // If we already have a bearer token and the claims are still valid
                // return the current bearer token
                if !is_expired && self.bearer_token.is_some() {
                    return Ok(self.bearer_token.clone().unwrap());
                } else if is_expired {
                    claims.refresh()?;
                }
                match &self.signing_key {
                    Some(signing_key) => {
                        self.bearer_token = Some(claims.encode_to(signing_key).await?);
                        Ok(self.bearer_token.clone().unwrap())
                    }
                    _ => Err(ApiError::auth_required()),
                }
            }
            // No claims, so no bearer token
            _ => match &self.bearer_token {
                Some(bearer_token) => Ok(bearer_token.clone()),
                _ => Err(ApiError::auth_required()),
            },
        }
    }

    /// Simple shortcut for checking if a user is authenticated
    pub async fn is_authenticated(&mut self) -> bool {
        self.bearer_token().await.is_ok()
    }

    /// Get the current subject based on the set credentials
    pub fn subject(&self) -> Result<String, ApiError> {
        match &self.claims {
            Some(claims) => {
                let sub = claims.sub()?;
                Ok(sub.to_string())
            }
            _ => Err(ApiError::auth_required()),
        }
    }

    /// Call a method that implements ApiRequest on the core server
    pub async fn call<T: ApiRequest>(&mut self, request: T) -> Result<T::ResponseType, ApiError> {
        // Determine if this request requires authentication
        let add_authentication = request.requires_authentication();
        let mut request_builder = request.build_request(&self.remote_core, &self.reqwest_client);

        if add_authentication {
            let bearer_token = self.bearer_token().await?;
            request_builder = request_builder.bearer_auth(bearer_token);
        }

        // Send the request and obtain the response
        let response = request_builder.send().await?;

        // If the call succeeded
        if response.status().is_success() {
            // Interpret the response as a JSON object
            response
                .json::<T::ResponseType>()
                .await
                .map_err(ApiError::format)
        } else {
            // If we got a 404
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                // Return a HTTP response error
                return Err(ApiError::http_response(response.status()));
            }

            // For other error responses, try to deserialize the error
            let err = response.json::<T::ErrorType>().await?;

            // Wrap the error
            let err = Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>;
            // Return Err
            Err(ApiError::from(err))
        }
    }

    /// Call a method that implements ApiRequest
    pub async fn call_no_content<T: ApiRequest>(&mut self, request: T) -> Result<(), ApiError> {
        let add_authentication = request.requires_authentication();
        let mut request_builder = request.build_request(&self.remote_core, &self.reqwest_client);
        if add_authentication {
            let bearer_token = self.bearer_token().await?;
            request_builder = request_builder.bearer_auth(bearer_token);
        }

        let response = request_builder.send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                // Handle 404 specifically
                // You can extend this part to handle other status codes differently if needed
                return Err(ApiError::http_response(response.status()));
            }
            // For other error responses, try to deserialize the error
            let err = response
                .json::<T::ErrorType>()
                .await
                .map_err(|err| ApiError::format(err))?;
            let err = Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>;
            Err(ApiError::from(err))
        }
    }

    /// Call a multipart method that implements ApiRequest
    // #[cfg(not(target_arch = "wasm32"))]
    pub async fn multipart<T: ApiRequest>(
        &mut self,
        request: T,
    ) -> Result<T::ResponseType, ApiError> {
        let add_authentication = request.requires_authentication();
        let mut request_builder = request.build_request(&self.remote_core, &self.reqwest_client);
        if add_authentication {
            let bearer_token = self.bearer_token().await?;
            request_builder = request_builder.bearer_auth(bearer_token);
        }

        let response = request_builder.send().await?;

        if response.status().is_success() {
            response
                .json::<T::ResponseType>()
                .await
                .map_err(ApiError::format)
        } else {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                // Handle 404 specifically
                // You can extend this part to handle other status codes differently if needed
                return Err(ApiError::http_response(response.status()));
            }
            // For other error responses, try to deserialize the error
            let err = response.json::<T::ErrorType>().await?;

            let err = Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>;
            Err(ApiError::from(err))
        }
    }

    /// Make a multipart request that returns no content
    pub async fn multipart_no_content<T: ApiRequest>(
        &mut self,
        request: T,
    ) -> Result<(), ApiError> {
        let add_authentication = request.requires_authentication();
        let mut request_builder = request.build_request(&self.remote_core, &self.reqwest_client);
        if add_authentication {
            let bearer_token = self.bearer_token().await?;
            request_builder = request_builder.bearer_auth(bearer_token);
        }

        let response = request_builder.send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                // Handle 404 specifically
                // You can extend this part to handle other status codes differently if needed
                return Err(ApiError::http_response(response.status()));
            }
            // For other error responses, try to deserialize the error
            let err = response.json::<T::ErrorType>().await?;

            let err = Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>;
            Err(ApiError::from(err))
        }
    }

    /// Stream a response from the API that implements StreamableApiRequest
    pub async fn stream<T: StreamableApiRequest>(
        &mut self,
        request: T,
        base_url: &Url,
    ) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, ApiError> {
        let add_authentication = request.requires_authentication();
        let mut request_builder = request.build_request(base_url, &self.reqwest_client);
        if add_authentication {
            let bearer_token = self.bearer_token().await?;
            request_builder = request_builder.bearer_auth(bearer_token);
        }

        let response = request_builder.send().await?;

        if response.status().is_success() {
            Ok(response.bytes_stream())
        } else {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                // Handle 404 specifically
                // You can extend this part to handle other status codes differently if needed
                return Err(ApiError::http_response(response.status()));
            }
            // For other error responses, try to deserialize the error
            let err = response.json::<T::ErrorType>().await?;

            let err = Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>;
            Err(ApiError::from(err))
        }
    }
}

// #[cfg(not(target_arch = "wasm32"))]
// fn multipart_headers(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
//     // Don't do anything!
//     request
// }
//
// #[cfg(target_arch = "wasm32")]
// fn multipart_headers(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
//     // Unset the content type header. The browser will set it automatically.
//     // If using in node environment ... 🤷‍♂️
//     request
//         .try_clone()
//         .expect("failed to clone request builder")
//         .header("Content-Type", "");
//     request
// }
