use std::{fmt::Display, path::PathBuf};

use thiserror::Error;
use tomb_common::banyan_api::error::ClientError;

#[derive(Error, Debug)]
#[non_exhaustive]
pub struct PipelineError {
    kind: PipelineErrorKind,
}

impl Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self.kind))
    }
}

impl PipelineError {
    pub fn client_error(err: ClientError) -> Self {
        Self {
            kind: PipelineErrorKind::Client(err),
        }
    }

    pub fn uninitialized_error(path: PathBuf) -> Self {
        Self {
            kind: PipelineErrorKind::Uninitialized(path),
        }
    }

    pub fn file_missing_error(path: PathBuf) -> Self {
        Self {
            kind: PipelineErrorKind::FileMissing(path),
        }
    }

    pub fn io_error(err: std::io::Error) -> Self {
        Self {
            kind: PipelineErrorKind::IoError(err),
        }
    }

    pub fn anyhow_error(err: anyhow::Error) -> Self {
        Self {
            kind: PipelineErrorKind::AnyhowError(err),
        }
    }
}

#[derive(Debug)]
pub enum PipelineErrorKind {
    // #[error("Client Error")]
    /// Error sending Client requests
    Client(ClientError),
    /// User simply never configured this directory
    // #[error("Bucket not been initialized for this directory: {0.display()}")]
    Uninitialized(PathBuf),
    // Missing File when searching for it during extracting
    // #[error("File not found in Content BlockStore: {0}")]
    FileMissing(PathBuf),
    /// IO Operation Error
    // #[error("Error performing IO operations: {:?}", .0)]
    IoError(std::io::Error),
    /// Anyhow errors
    // #[error("ANYHOW ERROR: {:?}", .0)]
    AnyhowError(anyhow::Error),
}

impl From<std::io::Error> for PipelineError {
    fn from(value: std::io::Error) -> Self {
        Self::io_error(value)
    }
}

impl From<anyhow::Error> for PipelineError {
    fn from(value: anyhow::Error) -> Self {
        Self::anyhow_error(value)
    }
}
