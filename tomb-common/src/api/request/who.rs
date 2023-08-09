use reqwest::Method;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::error::InfallibleError;

use super::Requestable;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhoRequest;

impl Requestable for WhoRequest {
    type ErrorType = InfallibleError;
    type ResponseType = WhoResponse;

    fn endpoint(&self) -> String {
        format!("/api/v1/auth/whoami")
    }
    fn method(&self) -> Method {
        Method::GET
    }
    fn authed(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
pub struct WhoResponse {
    pub account_id: Uuid,
}
