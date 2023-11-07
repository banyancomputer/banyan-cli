use std::error::Error;
use std::fmt::{self, Display, Formatter};
use tomb_crypt::prelude::TombCryptError;

/// Errors that can occur in the API Client
#[derive(Debug)]
#[non_exhaustive]
pub struct ClientError {
    #[allow(dead_code)]
    pub(crate) kind: ClientErrorKind,
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

    /// Custom error
    pub fn custom_error(message: &str) -> Self {
        Self {
            kind: ClientErrorKind::CustomError(message.to_string()),
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

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let prefix = match &self.kind {
            ClientErrorKind::ApiResponseError(err) => format!("API Response Error: {err}"),
            ClientErrorKind::AuthUnavailable => "Auth is required for this operation.".into(),
            ClientErrorKind::HttpClientError(_) => "HTTP Client Error".into(),
            ClientErrorKind::HttpResponseError(status_code) => {
                format!("HTTP Response Error: {status_code:?}")
            }
            ClientErrorKind::ResponseFormatError(_) => "Response Format Error".into(),
            ClientErrorKind::CryptoError(_) => "Cryptographic Error".into(),
            ClientErrorKind::CustomError(message) => message.into(),
        };

        write!(f, "{}", prefix)?;

        let mut next_err = self.source();
        while let Some(err) = next_err {
            write!(f, ": {err}")?;
            next_err = err.source();
        }

        Ok(())
    }
}

impl Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ClientErrorKind::HttpClientError(err) => Some(err),
            ClientErrorKind::ResponseFormatError(err) => Some(err),
            ClientErrorKind::CryptoError(err) => Some(err),
            _ => None,
        }
    }
}

/// The type of the Client Error
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
    /// CustomError
    CustomError(String),
}

impl From<anyhow::Error> for ClientError {
    fn from(value: anyhow::Error) -> Self {
        Self::custom_error(&value.to_string())
    }
}

impl From<TombCryptError> for ClientError {
    fn from(value: TombCryptError) -> Self {
        Self::crypto_error(value)
    }
}
