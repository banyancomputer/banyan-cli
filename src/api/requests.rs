use reqwest::{Client, RequestBuilder, Url};
use serde::de::DeserializeOwned;
use std::{error::Error, fmt::Debug};

/// API Request implementations for routes managed by the core service
pub mod core;
/// API Request implementations for managed by a storage host
pub mod staging;

/// Defintion of an API request
pub trait ApiRequest {
    /// Has a response type
    type ResponseType: DeserializeOwned;
    /// Has an error types
    type ErrorType: DeserializeOwned + Error + Send + Sync + Debug + 'static;

    /// Builds a Reqwest request
    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder;
    /// Optionally requires authentication
    fn requires_authentication(&self) -> bool;
}

/// Definition of a streamable API request
pub trait StreamableApiRequest {
    /// Has an error types
    type ErrorType: DeserializeOwned + Error + Send + Sync + Debug + 'static;

    /// Builds a Reqwest request
    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder;
    /// Optionally requires authentication
    fn requires_authentication(&self) -> bool;
}
