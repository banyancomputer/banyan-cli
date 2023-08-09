use super::{RequestMetadata, Requestable};
use crate::api::error::InfallibleError;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request to know who we are to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoRequest;

impl Requestable for WhoRequest {
    type ErrorType = InfallibleError;
    type ResponseType = WhoResponse;

    fn metadata(&self) -> RequestMetadata {
        RequestMetadata {
            endpoint: "/api/v1/auth/whoami".to_string(),
            method: Method::GET,
            auth: true,
        }
    }
}

/// The response given when asking Who Am I?
#[derive(Debug, Deserialize)]
pub struct WhoResponse {
    /// The account Id
    pub account_id: Uuid,
}
