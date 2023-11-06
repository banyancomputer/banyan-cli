use thiserror::Error;

/// Configuration errors.
#[derive(Debug, Error)]
pub(crate) enum SerialError {
    /// Missing metadata within Car file
    #[error("Missing {0} in metadata")]
    MissingMetadata(String),
    /// Node not found at path
    #[error("No node at path {0}")]
    NodeNotFound(String),
    // /// Node is file not directory
    // #[error("Node at path {0} is a file not a directory")]
    // NodeIsFile(String),
    // /// Node is directory not file
    // #[error("Node at path {0} is a directory not a file")]
    // NodeIsDirectory(String),
    #[error("The configuration file failed to deserialize correctly.")]
    _BadConfig,
}
