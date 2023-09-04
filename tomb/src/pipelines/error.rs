use std::{fmt::Display, path::PathBuf};
use thiserror::Error;
use tomb_common::banyan_api::error::ClientError;
use uuid::Uuid;

use crate::cli::command::BucketSpecifier;

/// Errors for the Tomb CLI & Native program
#[derive(Error, Debug)]
#[non_exhaustive]
pub struct TombError {
    kind: PipelineErrorKind,
}

impl Display for TombError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self.kind))
    }
}

impl TombError {
    /// Client Error
    pub fn client_error(err: ClientError) -> Self {
        Self {
            kind: PipelineErrorKind::Client(err),
        }
    }

    /// Unknown Bucket path
    pub fn unknown_path(path: PathBuf) -> Self {
        Self {
            kind: PipelineErrorKind::UnknownBucket(BucketSpecifier::with_origin(&path)),
        }
    }

    /// Unknown Bucket ID
    pub fn unknown_id(id: Uuid) -> Self {
        Self {
            kind: PipelineErrorKind::UnknownBucket(BucketSpecifier::with_id(id)),
        }
    }

    /// Unable to find Node in CAR
    pub fn file_missing_error(path: PathBuf) -> Self {
        Self {
            kind: PipelineErrorKind::FileMissing(path),
        }
    }

    /// Error performing IO operations
    pub fn io_error(err: std::io::Error) -> Self {
        Self {
            kind: PipelineErrorKind::IoError(err),
        }
    }

    /// Anyhow errors
    pub fn anyhow_error(err: anyhow::Error) -> Self {
        Self {
            kind: PipelineErrorKind::AnyhowError(err),
        }
    }
}

/// Pipelin Error
#[derive(Debug)]
pub enum PipelineErrorKind {
    /// Error sending Client requests
    Client(ClientError),
    /// User simply never configured this directory
    UnknownBucket(BucketSpecifier),
    /// Missing File when searching for it during extracting
    FileMissing(PathBuf),
    /// IO Operation Error
    IoError(std::io::Error),
    /// Anyhow errors
    AnyhowError(anyhow::Error),
}

impl From<std::io::Error> for TombError {
    fn from(value: std::io::Error) -> Self {
        Self::io_error(value)
    }
}

impl From<anyhow::Error> for TombError {
    fn from(value: anyhow::Error) -> Self {
        Self::anyhow_error(value)
    }
}
