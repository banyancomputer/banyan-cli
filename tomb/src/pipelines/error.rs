use std::{error::Error, fmt::Display, path::PathBuf};
use thiserror::Error;
use tomb_common::banyan_api::error::ClientError;
use uuid::Uuid;

use crate::cli::specifiers::BucketSpecifier;

/// Errors for the Tomb CLI & Native program
#[derive(Error, Debug)]
#[non_exhaustive]
pub struct TombError {
    kind: TombErrorKind,
}

impl TombError {
    /// Client Error
    pub fn client_error(err: ClientError) -> Self {
        Self {
            kind: TombErrorKind::Client(err),
        }
    }

    /// Unknown Bucket path
    pub fn unknown_path(path: PathBuf) -> Self {
        Self {
            kind: TombErrorKind::UnknownBucket(BucketSpecifier::with_origin(&path)),
        }
    }

    /// Unknown Bucket ID
    pub fn unknown_id(id: Uuid) -> Self {
        Self {
            kind: TombErrorKind::UnknownBucket(BucketSpecifier::with_id(id)),
        }
    }

    /// Unable to find Node in CAR
    pub fn file_missing_error(path: PathBuf) -> Self {
        Self {
            kind: TombErrorKind::FileMissing(path),
        }
    }

    /// Error performing IO operations
    pub fn io_error(err: std::io::Error) -> Self {
        Self {
            kind: TombErrorKind::IoError(err),
        }
    }

    /// Anyhow errors
    pub fn anyhow_error(err: anyhow::Error) -> Self {
        Self {
            kind: TombErrorKind::AnyhowError(err),
        }
    }
}

/// Pipelin Error
#[derive(Debug)]
pub enum TombErrorKind {
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

impl Display for TombError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TombErrorKind::*;
        let prefix = match &self.kind {
            Client(err) => format!("client error: {err}"),
            UnknownBucket(bucket) => format!("couldnt find bucket: {:?}", bucket),
            FileMissing(path) => format!("missing file at path: {}", path.display()),
            IoError(err) => format!("io error: {err}"),
            AnyhowError(err) => format!("anyhow error: {err}"),
        };

        write!(f, "{}", prefix)?;

        let mut next_err = self.source();
        while let Some(err) = next_err {
            write!(f, ": {err}")?;
            next_err = err.source();
        }

        Ok(())
    }
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

impl From<ClientError> for TombError {
    fn from(value: ClientError) -> Self {
        Self::client_error(value)
    }
}
