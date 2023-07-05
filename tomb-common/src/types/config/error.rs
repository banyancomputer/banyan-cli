use thiserror::Error;

/// Configuration errors.
#[derive(Debug, Error)]
pub(crate) enum ConfigError {
    #[error("Missing {0} in metadata")]
    MissingMetadata(String),
    #[error("Remote URL specified is not in a valid format: {0}")]
    BadEndpoint(String),
    #[error("The configuration file failed to deserialize correctly.")]
    BadConfig
}
