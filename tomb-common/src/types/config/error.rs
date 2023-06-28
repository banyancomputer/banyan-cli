use thiserror::Error;

/// Configuration errors.
#[derive(Debug, Error)]
pub(crate) enum ConfigError {
    #[error("Unable to find {0} in Metadata")]
    MissingMetadata(String),
    #[error("Remote URL specified is not in a valid format: {0}")]
    BadEndpoint(String),
}
