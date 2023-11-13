use crate::api::error::ClientError;
use colored::Colorize;
use std::{error::Error, fmt::Display, path::PathBuf};
use thiserror::Error;
use tomb_crypt::prelude::TombCryptError;

#[cfg(feature = "cli")]
use {crate::cli::specifiers::DriveSpecifier, uuid::Uuid};

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
    #[cfg(feature = "cli")]
    pub fn unknown_path(path: PathBuf) -> Self {
        Self {
            kind: TombErrorKind::UnknownBucket(DriveSpecifier::with_origin(&path)),
        }
    }

    /// Unknown Bucket ID
    #[cfg(feature = "cli")]
    pub fn unknown_id(id: Uuid) -> Self {
        Self {
            kind: TombErrorKind::UnknownBucket(DriveSpecifier::with_id(id)),
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
    pub fn custom_error(msg: &str) -> Self {
        Self {
            kind: TombErrorKind::CustomError(msg.to_string()),
        }
    }
}

/// Pipelin Error
#[derive(Debug)]
pub enum TombErrorKind {
    /// Error sending Client requests
    Client(ClientError),
    /// User simply never configured this directory
    #[cfg(feature = "cli")]
    UnknownBucket(DriveSpecifier),
    /// Missing File when searching for it during restoring
    FileMissing(PathBuf),
    /// IO Operation Error
    IoError(std::io::Error),
    /// Custom errors
    CustomError(String),
}

impl Display for TombError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = match &self.kind {
            TombErrorKind::Client(err) => format!("{} {err}", "CLIENT ERROR:".underline()),
            #[cfg(feature = "cli")]
            TombErrorKind::UnknownBucket(bucket) => format!("couldnt find bucket: {:?}", bucket),
            TombErrorKind::FileMissing(path) => format!("missing file at path: {}", path.display()),
            TombErrorKind::IoError(err) => format!("{} {err}", "IO ERROR:".underline()),
            TombErrorKind::CustomError(err) => err.to_string(),
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
        Self::custom_error(&value.to_string())
    }
}

impl From<ClientError> for TombError {
    fn from(value: ClientError) -> Self {
        Self::client_error(value)
    }
}

impl From<TombCryptError> for TombError {
    fn from(value: TombCryptError) -> Self {
        Self::client_error(ClientError::crypto_error(value))
    }
}