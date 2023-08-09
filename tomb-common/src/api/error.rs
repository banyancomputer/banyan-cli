use std::fmt::{Display, Formatter};

use serde::Deserialize;
use thiserror::Error;

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
}

impl From<Box<dyn std::error::Error + Send + Sync + 'static>> for ClientError {
    fn from(err: Box<dyn std::error::Error + Send + Sync + 'static>) -> Self {
        Self {
            kind: ClientErrorKind::ApiResponseError(err),
        }
    }
}

// impl Display for ClientError {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         match &self.kind {
//             ClientErrorKind::ApiResponseError(_) => todo!(),
//             ClientErrorKind::AuthUnavailable => todo!(),
//             ClientErrorKind::HttpClientError(_) => todo!(),
//             ClientErrorKind::ResponseFormatError(err) => {
//                 f.write_str(err.fmt(f))
//             },
//         }
//     }
// }

#[derive(Debug)]
#[non_exhaustive]
enum ClientErrorKind {
    ApiResponseError(Box<dyn std::error::Error + Send + Sync + 'static>),
    AuthUnavailable,
    HttpClientError(reqwest::Error),
    ResponseFormatError(reqwest::Error),
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
    status: String
}
impl Display for StatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.status)
    }
}