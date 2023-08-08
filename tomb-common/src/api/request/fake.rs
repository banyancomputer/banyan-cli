use std::{
    error::Error,
    fmt::{Display, Formatter},
};

use super::{Requestable, Respondable};
use crate::api::error::InfallibleError;
use async_trait::async_trait;
use reqwest::{Method, Response};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub enum FakeRequest {
    RegisterAccount,
    RegisterDeviceKey(FakeRegisterDeviceKeyRequest),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FakeRegisterDeviceKeyRequest {
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

impl Requestable for FakeRequest {
    fn endpoint(&self) -> String {
        match self {
            FakeRequest::RegisterAccount => format!("/api/v1/auth/create_fake_account"),
            FakeRequest::RegisterDeviceKey(_) => format!("/api/v1/auth/fake_register_device_key"),
        }
    }

    fn method(&self) -> reqwest::Method {
        match self {
            FakeRequest::RegisterAccount => Method::GET,
            FakeRequest::RegisterDeviceKey(_) => Method::POST,
        }
    }

    fn authed(&self) -> bool {
        false
    }
}
