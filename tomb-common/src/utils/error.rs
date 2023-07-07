
use thiserror::Error;

/// Configuration errors.
#[derive(Debug, Error)]
pub(crate) enum SerialError {
    #[error("Missing {0} in metadata")]
    MissingMetadata(String),
    #[error("The configuration file failed to deserialize correctly.")]
    _BadConfig,
}
