use std::error::Error;

use reqwest::{Client, RequestBuilder, Url};
use serde::de::DeserializeOwned;

pub mod auth;
pub mod buckets;

pub trait ApiRequest {
    type ResponseType: DeserializeOwned;
    type ErrorType: DeserializeOwned + Error + Send + Sync + 'static;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder;
    fn requires_authentication(&self) -> bool;
}
