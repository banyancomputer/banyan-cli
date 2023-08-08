use std::{
    error::Error,
    fmt::{Display, Formatter},
};

use super::Requestable;
use crate::api::error::InfallibleError;
use async_trait::async_trait;
use reqwest::{Method, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterAccountRequest;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterDeviceKeyRequest {
    pub token: String,
    pub public_key: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterAccountResponse {
    pub id: Uuid,
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterDeviceKeyResponse {
    pub id: Uuid,
    pub account_id: Uuid,
    pub fingerprint: String,
}

impl Requestable for RegisterAccountRequest {
    type ResponseType = RegisterAccountResponse;
    type ErrorType = InfallibleError;
    fn endpoint(&self) -> String {
        format!("/api/v1/auth/create_fake_account")
    }
    fn method(&self) -> Method {
        Method::GET
    }
    fn authed(&self) -> bool {
        false
    }
}

impl Requestable for RegisterDeviceKeyRequest {
    type ResponseType = RegisterDeviceKeyResponse;
    type ErrorType = InfallibleError;
    fn endpoint(&self) -> String {
        format!("/api/v1/auth/fake_register_device_key")
    }
    fn method(&self) -> Method {
        Method::POST
    }
    fn authed(&self) -> bool {
        false
    }
}
