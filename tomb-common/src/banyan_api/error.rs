use std::error::Error;
use std::fmt::{self, Display, Formatter};

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

impl Display for ClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ClientErrorKind::*;

        let prefix = match &self.kind {
            ApiResponseError(err) => format!("api response error: {err}"),
            AuthUnavailable => "auth unavailable".into(),
            HttpClientError(_) => "http client error".into(),
            HttpResponseError(status_code) => format!("http response error: {status_code:?}"),
            ResponseFormatError(_) => "response format error".into(),
            CryptoError(_) => "crypto error".into(),
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
        use ClientErrorKind::*;

        match &self.kind {
            HttpClientError(err) => Some(err),
            ResponseFormatError(err) => Some(err),
            CryptoError(err) => Some(err),
            _ => None,
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
