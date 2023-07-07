use std::path::PathBuf;
use thiserror::Error;

/// CAR File errors.
#[derive(Debug, Error)]
pub enum CarError {
    #[error("CAR path given was directory, not file: {}", .0.display())]
    Directory(PathBuf),
    #[error("Unable to save CAR file to disk.")]
    FailToSave,
    #[error("Failed to load CAR file from disk: {}", .0.display())]
    FailToLoad(PathBuf),
    #[error("CAR v1 Had a malformed header")]
    MalformedV1Header,
}
