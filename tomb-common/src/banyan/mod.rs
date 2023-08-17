/// Our API request implementation
#[cfg(feature = "api")]
#[allow(missing_docs)]
pub mod api;
/// Our API client
#[cfg(feature = "api")]
pub mod client;
/// Our API models, along with CRUD implementations
pub mod models;

mod credentials;
#[cfg(feature = "api")]
mod error;
