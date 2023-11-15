use std::error::Error;
use std::fmt::{self, Display, Formatter};
use tomb_crypt::prelude::TombCryptError;
use url::ParseError;

/// Errors that can occur in the API Client
#[derive(Debug)]
#[non_exhaustive]
pub struct ApiError {
    kind: ApiErrorKind,
}

impl ApiError {
    /// Authentication is not available
    pub fn auth_required() -> Self {
        Self {
            kind: ApiErrorKind::AuthUnavailable,
        }
    }

    /// Response format was invalid
    pub fn format(err: reqwest::Error) -> Self {
        Self {
            kind: ApiErrorKind::ResponseFormat(err),
        }
    }

    pub fn reqwest_general(err: reqwest::Error) -> Self {
        Self {
            kind: ApiErrorKind::ReqwestGeneral(err),
        }
    }

    pub fn http_response(status: reqwest::StatusCode) -> Self {
        Self {
            kind: ApiErrorKind::HttpResponse(status),
        }
    }

    /// HTTP error
    pub fn http(err: reqwest::Error) -> Self {
        Self {
            kind: ApiErrorKind::HttpClient(err),
        }
    }

    /// Cryptography error
    pub fn crypto(err: TombCryptError) -> Self {
        Self {
            kind: ApiErrorKind::Crypto(err),
        }
    }

    pub fn parse(err: ParseError) -> Self {
        Self {
            kind: ApiErrorKind::Parse(err),
        }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for ApiError {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self {
            kind: ApiErrorKind::ApiResponse(err),
        }
    }
}

impl Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let prefix = match &self.kind {
            ApiErrorKind::ApiResponse(err) => format!("API Response Error: {err}"),
            ApiErrorKind::AuthUnavailable => "Auth is required for this operation.".into(),
            ApiErrorKind::HttpClient(_) => "HTTP Client Error".into(),
            ApiErrorKind::HttpResponse(status_code) => {
                format!("HTTP Response Error: {status_code:?}")
            }
            ApiErrorKind::ResponseFormat(_) => "Response Format Error".into(),
            ApiErrorKind::Crypto(_) => "Cryptographic Error".into(),
            ApiErrorKind::ReqwestGeneral(_) => todo!(),
            ApiErrorKind::Parse(_) => todo!(),
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

impl Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.kind {
            ApiErrorKind::HttpClient(err) => Some(err),
            ApiErrorKind::ResponseFormat(err) => Some(err),
            ApiErrorKind::Crypto(err) => Some(err),
            _ => None,
        }
    }
}

/// The type of the Client Error
#[derive(Debug)]
enum ApiErrorKind {
    /// API Response Error
    ApiResponse(Box<dyn std::error::Error + Send + Sync + 'static>),
    /// Authentication is not available
    AuthUnavailable,
    ReqwestGeneral(reqwest::Error),
    /// HTTP error
    HttpClient(reqwest::Error),
    /// HTTP Response indicated error
    HttpResponse(reqwest::StatusCode),
    /// Response format was invalid
    ResponseFormat(reqwest::Error),
    /// Cryptography error
    Crypto(TombCryptError),
    /// CustomError
    Parse(ParseError),
}

impl From<TombCryptError> for ApiError {
    fn from(value: TombCryptError) -> Self {
        Self::crypto(value)
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(value: reqwest::Error) -> Self {
        Self::reqwest_general(value)
    }
}

impl From<ParseError> for ApiError {
    fn from(value: ParseError) -> Self {
        Self::parse(value)
    }
}
