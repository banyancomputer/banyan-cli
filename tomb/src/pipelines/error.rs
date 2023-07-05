use thiserror::Error;

/// Pipeline errors.
#[derive(Debug, Error)]
pub enum PipelineError {
    /// User simply never configured this directory
    #[error("Directory has not been initialized")]
    Uninitialized,

    /// Missing File when searching for it during unpacking
    #[error("File not found in Content BlockStore: {0}")]
    FileNotFound(String),

    /// io Errors
    #[error("Error performing IO operations: {:?}", .0)]
    IoError(std::io::Error),

    /// Anyhow errors
    #[error("ANYHOW ERROR: {:?}", .0)]
    AnyhowError(anyhow::Error),
}

impl From<std::io::Error> for PipelineError {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<anyhow::Error> for PipelineError {
    fn from(value: anyhow::Error) -> Self {
        Self::AnyhowError(value)
    }
}
