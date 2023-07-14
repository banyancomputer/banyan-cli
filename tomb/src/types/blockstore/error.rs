use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
/// Errors for singular CAR files
pub enum SingleError {
    #[error("CAR given was directory, not file: {}", .0.display())]
    /// Expected file got dir
    Directory(PathBuf),
    #[error("Unable to save CAR file to disk.")]
    /// Unable to save file
    FailToSave,
    #[error("Failed to load CAR file from disk: {}", .0.display())]
    /// Unable to load file
    FailToLoad(PathBuf),
}

#[derive(Debug, Error)]
/// Errors for groups of CAR files
pub enum MultiError {
    #[error("Path given was file, not directory: {}", .0.display())]
    /// Expected dir got file
    File(PathBuf),
}

#[derive(Debug, Error)]
/// File-based CAR errors.
pub enum CARIOError {
    /// Errors for singular CAR files
    #[error("Singular: {0}")]
    SingleError(#[from] SingleError),
    /// Errors for groups of CAR files
    #[error("Multi: {0}")]
    MultiError(#[from] MultiError),
}
