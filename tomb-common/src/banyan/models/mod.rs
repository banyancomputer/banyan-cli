use serde::Deserialize;
use std::error::Error;
use std::fmt::Debug;
use std::fmt::{self, Display, Formatter};

pub mod account;
pub mod bucket;
pub mod bucket_key;
pub mod device_api_key;
pub mod bucket_metadata;
pub mod storage_ticket;

// TODO: bubble up errors from the client

/// A generic error type for Models that can be returned from the API
#[derive(Debug, Deserialize)]
pub struct ModelError {
    #[serde(rename = "error")]
    kind: ModelErrorKind,
}

impl ModelError {
    pub fn unknown() -> Self {
        Self {
            kind: ModelErrorKind::Unknown,
        }
    }
    pub fn missing_id() -> Self {
        Self {
            kind: ModelErrorKind::MissingId,
        }
    }
    pub fn missing_field(field: String) -> Self {
        Self {
            kind: ModelErrorKind::MissingField(field),
        }
    }
    pub fn unsupported_request() -> Self {
        Self {
            kind: ModelErrorKind::UnsupportedRequest,
        }
    }
    pub fn client_error() -> Self {
        Self {
            kind: ModelErrorKind::ClientError,
        }
    }
}

impl Display for ModelError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ModelErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred",
            MissingId => "the requested resource does not have an ID",
            MissingField(_) => "the requested resource is missing a required field",
            UnsupportedRequest => "the requested resource does not support this request",
            ClientError => "a client error occurred",
        };

        f.write_str(msg)
    }
}

impl Error for ModelError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModelErrorKind {
    Unknown,
    ClientError,
    MissingId,
    MissingField(String),
    UnsupportedRequest,
}
