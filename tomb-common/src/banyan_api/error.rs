use std::fmt::Display;

use thiserror::Error;
use tomb_crypt::prelude::TombCryptError;

/// Errors that can occur in the API Client
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
    /// Authentication is not available
    pub fn auth_unavailable() -> Self {
        Self {
            kind: ClientErrorKind::AuthUnavailable,
        }
    }

    /// Response format was invalid
    pub fn bad_format(err: reqwest::Error) -> Self {
        Self {
            kind: ClientErrorKind::ResponseFormatError(err),
        }
    }

    /// HTTP Response indicated error
    pub fn http_response_error(status: reqwest::StatusCode) -> Self {
        Self {
            kind: ClientErrorKind::HttpResponseError(status),
        }
    }

    /// HTTP error
    pub fn http_error(err: reqwest::Error) -> Self {
        Self {
            kind: ClientErrorKind::HttpClientError(err),
        }
    }

    /// Cryptography error
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

/// Kind of ClientError
#[derive(Debug)]
#[non_exhaustive]
pub enum ClientErrorKind {
    /// API Response Error
    ApiResponseError(Box<dyn std::error::Error + Send + Sync + 'static>),
    /// Authentication is not available
    AuthUnavailable,
    /// HTTP error
    HttpClientError(reqwest::Error),
    /// HTTP Response indicated error
    HttpResponseError(reqwest::StatusCode),
    /// Response format was invalid
    ResponseFormatError(reqwest::Error),
    /// Cryptography error
    CryptoError(TombCryptError),
}
