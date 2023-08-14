use async_trait::async_trait;
use std::fmt::Debug;
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use serde::Deserialize;

use crate::banyan::client::Client;

pub mod account;
pub mod device_api_key;
// pub mod bucket;
// pub mod bucket_metadata;
// pub mod bucket_key;

#[async_trait(?Send)]
/// Implement this trait to make a model or data structure requestable against the Banyan API with a client.
/// This basically flattens backend management into a CRUD interface.
/// Don't be surprised if you see the same model being requested from various endpoints.
/// Non-implemented methods reflect endpoints that don't exist.
pub trait Requestable: Sized + Debug + DeserializeOwned {
    type ErrorType;

    /// Get the ID of this model or data structure
    fn id(&self) -> Result<uuid::Uuid, Self::ErrorType>;

    // TODO: This shold take a generic serialiable object and return an instance of the model. The current implementation is a bit of a hack and annoying to use.
    /// Create a new instance of this model or data structure. Returns the instance with the ID attached
    async fn create(self: Self,client: &mut Client) -> Result<Self, Self::ErrorType>;

    /// Read all instances of this model or data structure. Returns a vector of instances
    async fn read_all(client: &mut Client) -> Result<Vec<Self>, Self::ErrorType>;

    /// Read a single instance of this model or data structure. Returns the instance
    async fn read(client: &mut Client, id: &str) -> Result<Self, Self::ErrorType>;

    /// Update a single instance of this model or data structure. Returns the instance
    async fn update(self: Self, client: &mut Client, id: &str) -> Result<Self, Self::ErrorType>;

    /// Delete a single instance of this model or data structure. Returns the instance
    async fn delete(client: &mut Client, id: &str) -> Result<Self, Self::ErrorType>;
}

// TODO: bubble up errors from the client

/// A generic error type for API requests that have no error state at the application level (client errors may still occur).
#[derive(Debug, Deserialize)]
pub struct RequestableError {
    #[serde(rename = "error")]
    kind: RequestableErrorKind,
}

impl RequestableError {
    pub fn unknown() -> Self {
        Self {
            kind: RequestableErrorKind::Unknown,
        }
    }
    pub fn missing_id() -> Self {
        Self {
            kind: RequestableErrorKind::MissingId,
        }
    }
    pub fn missing_field(field: String) -> Self {
        Self {
            kind: RequestableErrorKind::MissingField(field),
        }
    }
    pub fn unsupported_request() -> Self {
        Self {
            kind: RequestableErrorKind::UnsupportedRequest,
        }
    }
    pub fn client_error() -> Self {
        Self {
            kind: RequestableErrorKind::ClientError,
        }
    }
}

impl Display for RequestableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use RequestableErrorKind::*;

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

impl Error for RequestableError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestableErrorKind {
    Unknown,
    ClientError,
    MissingId,
    MissingField(String),
    UnsupportedRequest,
}