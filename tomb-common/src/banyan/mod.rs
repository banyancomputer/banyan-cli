/// Our API request implementation
#[cfg(feature = "banyan-api")]
#[allow(missing_docs)]
pub mod api;
/// Our API client
#[cfg(feature = "banyan-api")]
pub mod client;
/// Our API models, along with CRUD implementations (when banyan-api is enabled)
pub mod models;

mod credentials;
#[cfg(feature = "banyan-api")]
mod error;
