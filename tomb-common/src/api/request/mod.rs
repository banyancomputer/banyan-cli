use clap::Subcommand;
use reqwest::Method;
use serde::{de::DeserializeOwned, Serialize};
use std::error::Error;

mod bucket;
mod key;
mod metadata;
mod who;

#[cfg(test)]
pub mod fake;

pub use bucket::*;
pub use key::*;
pub use metadata::*;
pub use who::*;

/// Metadata necessary to create a request
#[derive(Debug)]
pub struct RequestMetadata {
    /// The API endpoint which we are speaking to
    pub endpoint: String,
    /// The Method of speaking
    pub method: Method,
    /// The need or lack thereof for authenticating this request
    pub auth: bool,
}

/// An enum or struct which can be used to crate a request
pub trait Requestable: Serialize + Sized {
    /// The Error that the server will return on the failure of this request
    type ErrorType: DeserializeOwned + Error + Send + Sync + 'static;
    /// The Response that the server will return on the success of this request
    type ResponseType: DeserializeOwned;
    /// Metadata associated with the request
    fn metadata(&self) -> RequestMetadata;
}

/// A request to the Metadata API
#[derive(Clone, Debug, Subcommand)]
pub enum Request {
    /// Create, Delete, or get info on Buckets
    Bucket {
        /// Bucket Subcommand
        #[clap(subcommand)]
        subcommand: BucketRequest,
    },
    /// Create, Delete, or get info on Keys
    Keys {
        /// Key Subcommand
        #[clap(subcommand)]
        subcommand: KeyRequest,
    },
    /// Create, Delete, or get info on Metadata
    Metadata {
        /// Metadata Subcommand
        #[clap(subcommand)]
        subcommand: MetadataRequest,
    },
}
