use std::{fmt::{Display, Formatter}, error::Error};

use async_trait::async_trait;
use reqwest::{Method, Response};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;
use crate::api::error::InfallibleError;
use super::{Requestable, Respondable};

#[derive(Debug, Serialize, Deserialize)]
pub enum FakeRequest {
    RegisterAccount,
    RegisterDeviceKey(FakeRegisterDeviceKeyRequest)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FakeRegisterDeviceKeyRequest {
    pub public_key: String,
}

pub enum FakeResponse {
    Account(RegisterAccountResponse),
    Device(RegisterDeviceKeyResponse)
}

pub enum FakeError {
    Account(InfallibleError),
    Device(FakeRegisterDeviceKeyError)
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct FakeRegisterDeviceKeyError {
    #[serde(rename = "error")]
    kind: FakeRegisterDeviceKeyErrorKind,
}

impl Display for FakeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use FakeRegisterDeviceKeyErrorKind::*;

        let msg = match &self.kind {
            InvalidPublicKey => "provided public key was invalid",
            KeyContextUnavailable => "key context was unavailable to process key",
            PersistenceFailed => "unable to persist changes on the server side",
        };

        f.write_str(msg)
    }
}

impl Error for FakeError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum FakeRegisterDeviceKeyErrorKind {
    InvalidPublicKey,
    KeyContextUnavailable,
    PersistenceFailed,
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
    type ResponseType = FakeResponse;
    type ErrorType = FakeError;

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

#[async_trait(?Send)]
impl Respondable<FakeRequest, FakeError> for FakeResponse {
    async fn process(request: FakeRequest, response: reqwest::Response) -> Result<Self, FakeError> {
        match request {
            FakeRequest::RegisterAccount => {
                Ok(Self::Account(response.json::<RegisterAccountResponse>().await?))
            },
            FakeRequest::RegisterDeviceKey(_) => {
                Ok(Self::Device(response.json::<RegisterDeviceKeyResponse>().await?))
            },
        }
    }
}