use async_trait::async_trait;
use clap::{Subcommand, ValueEnum};
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

pub struct RequestMetadata {
    pub endpoint: String,
    pub method: Method,
    pub auth: bool
}

/// An enum or struct which can be used to crate a request
pub trait Requestable: Serialize + Sized {
    type ErrorType: DeserializeOwned + Error + Send + Sync + 'static;
    type ResponseType: DeserializeOwned;

    // Obtain the url suffix of the endpoint
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
