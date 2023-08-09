use reqwest::Method;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::error::InfallibleError;

use super::{Requestable, RequestMetadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoRequest;

impl Requestable for WhoRequest {
    type ErrorType = InfallibleError;
    type ResponseType = WhoResponse;
    
    fn metadata(&self) -> RequestMetadata {
        RequestMetadata {
            endpoint: format!("/api/v1/auth/whoami"),
            method: Method::GET,
            auth: true,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct WhoResponse {
    pub account_id: Uuid,
}
