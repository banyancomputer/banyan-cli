/// Bucket level configuration
pub mod bucket;
/// API Endpoint config
mod endpoints;
/// Global level configurations
pub mod globalconfig;
/// Key config
pub mod keys;
/// XDG config
pub mod xdg;
pub use endpoints::Endpoints;

mod error;

pub(crate) use error::ConfigurationError;
