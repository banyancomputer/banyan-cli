use async_trait::async_trait;
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

#[async_trait(?Send)]
pub trait Requestable: Serialize + Sized {
    type ErrorType: DeserializeOwned + Error + Send + Sync + 'static;
    type ResponseType: DeserializeOwned;

    // Obtain the url suffix of the endpoint
    fn endpoint(&self) -> String;
    fn method(&self) -> Method;
    fn authed(&self) -> bool;
}

pub enum Request {
    /// Set the remote endpoint where buckets are synced to / from
    Bucket(BucketRequest),
    /// Set the remote endpoint where buckets are synced to / from
    Keys(KeyRequest),
    /// Set the remote endpoint where buckets are synced to / from
    Metadata(MetadataRequest),
}
