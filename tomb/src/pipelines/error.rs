use thiserror::Error;

/// Pipeline errors.
#[derive(Debug, Error)]
pub(crate) enum PipelineError {
    #[error("Directory has not been initialized")]
    Uninitialized,
    #[error("File not found in Content BlockStore")]
    FileNotFound,
}
