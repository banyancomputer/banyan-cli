use std::fmt::Display;

use thiserror::Error;
use tomb_crypt::prelude::TombCryptError;
#[derive(Error, Debug)]
#[non_exhaustive]
pub struct ClientError {
    #[allow(dead_code)]
    kind: ClientErrorKind,
}

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self.kind))
    }
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

    pub fn http_response_error(status: reqwest::StatusCode) -> Self {
        Self {
            kind: ClientErrorKind::HttpResponseError(status),
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
    HttpResponseError(reqwest::StatusCode),
    ResponseFormatError(reqwest::Error),
    CryptoError(TombCryptError),
}
