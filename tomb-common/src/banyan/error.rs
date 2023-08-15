use std::fmt::{Display, Formatter};

use serde::Deserialize;
use thiserror::Error;
use tomb_crypt::prelude::TombCryptError;

#[derive(Debug)]
#[non_exhaustive]
pub struct ClientError {
    #[allow(dead_code)]
    kind: ClientErrorKind,
}

impl ClientError {
    pub fn auth_unavailable() -> Self {
        Self {
            kind: ClientErrorKind::AuthUnavailable,
        }
    }

    pub fn bad_format(err: reqwest::Error) -> Self {
        Self {
            kind: ClientErrorKind::ResponseFormatError(err),
        }
    }

    pub fn http_error(err: reqwest::Error) -> Self {
        Self {
            kind: ClientErrorKind::HttpClientError(err),
        }
    }

    pub fn crypto_error(err: TombCryptError) -> Self {
        Self {
            kind: ClientErrorKind::CryptoError(err),
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for ClientError {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self {
            kind: ClientErrorKind::ApiResponseError(err),
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub enum ClientErrorKind {
    ApiResponseError(Box<dyn std::error::Error + Send + Sync + 'static>),
    AuthUnavailable,
    HttpClientError(reqwest::Error),
    ResponseFormatError(reqwest::Error),
    CryptoError(TombCryptError),
}

#[derive(Debug, Deserialize)]
pub struct InfallibleError;

impl Display for InfallibleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("an infallible API query returned a failed response")
    }
}

impl std::error::Error for InfallibleError {}

#[derive(Debug, Error, Deserialize)]
pub struct StatusError {
    status: String,
}
impl Display for StatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.status)
    }
}
