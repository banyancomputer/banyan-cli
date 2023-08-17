use std::error::Error;

use reqwest::{Client, RequestBuilder, Url};
use serde::de::DeserializeOwned;

/// API Request implementations for routes under api/v1/auth
pub mod auth;
/// API Request implementations for routes under api/v1/buckets
pub mod buckets;

/// Defintion of an API request
pub trait ApiRequest {
    /// Has a response type
    type ResponseType: DeserializeOwned;
    /// Has an error types
    type ErrorType: DeserializeOwned + Error + Send + Sync + 'static;

    /// Builds a Reqwest request
    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder;
    /// Optionally requires authentication
    fn requires_authentication(&self) -> bool;
}

/// Definition of a streamable API request
pub trait StreamableApiRequest {
    /// Has an error types
    type ErrorType: DeserializeOwned + Error + Send + Sync + 'static;

    /// Builds a Reqwest request
    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder;
    /// Optionally requires authentication
    fn requires_authentication(&self) -> bool;
}
