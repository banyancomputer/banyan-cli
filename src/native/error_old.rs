// use crate::api::error::ApiError;
// use colored::Colorize;
// use std::{error::Error, fmt::Display, path::PathBuf};
// use thiserror::Error;
// use tomb_crypt::prelude::TombCryptError;

// use super::sync::SyncError;

// #[cfg(feature = "cli")]
// use {crate::cli::specifiers::DriveSpecifier, uuid::Uuid};

// /// Errors for the Tomb CLI & Native program
// #[derive(Error, Debug)]
// #[non_exhaustive]
// pub struct SyncError {
//     kind: NativeErrorKind,
// }

// impl SyncError {
    

//     /// Unable to find Node in CAR
//     pub fn file_missing_error(path: PathBuf) -> Self {
//         Self {
//             kind: NativeErrorKind::FileMissing(path),
//         }
//     }

//     /// Error performing IO operations
//     pub fn io_error(err: std::io::Error) -> Self {
//         Self {
//             kind: NativeErrorKind::IoError(err),
//         }
//     }

//     /// Anyhow errors
//     pub fn custom_error(msg: &str) -> Self {
//         Self {
//             kind: NativeErrorKind::CustomError(msg.to_string()),
//         }
//     }
// }

// /// Pipelin Error
// #[derive(Debug)]
// pub enum NativeErrorKind {
//     /// Error sending Client requests
//     Api(ApiError),
//     /// User simply never configured this directory
//     #[cfg(feature = "cli")]
//     UnknownBucket(DriveSpecifier),
//     /// Missing File when searching for it during restoring
//     FileMissing(PathBuf),
//     /// IO Operation Error
//     IoError(std::io::Error),
//     /// Custom errors
//     CustomError(String),
// }

// impl Display for SyncError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let prefix = match &self.kind {
//             NativeErrorKind::Api(err) => format!("{} {err}", "CLIENT ERROR:".underline()),
//             #[cfg(feature = "cli")]
//             NativeErrorKind::UnknownBucket(bucket) => format!("couldnt find bucket: {:?}", bucket),
//             NativeErrorKind::FileMissing(path) => format!("missing file at path: {}", path.display()),
//             NativeErrorKind::IoError(err) => format!("{} {err}", "IO ERROR:".underline()),
//             NativeErrorKind::CustomError(err) => err.to_string(),
//         };

//         write!(f, "{}", prefix)?;

//         let mut next_err = self.source();
//         while let Some(err) = next_err {
//             write!(f, ": {err}")?;
//             next_err = err.source();
//         }

//         Ok(())
//     }
// }

// impl From<std::io::Error> for SyncError {
//     fn from(value: std::io::Error) -> Self {
//         Self::io_error(value)
//     }
// }

// impl From<anyhow::Error> for SyncError {
//     fn from(value: anyhow::Error) -> Self {
//         Self::custom_error(&value.to_string())
//     }
// }

// impl From<ApiError> for SyncError {
//     fn from(value: ApiError) -> Self {
//         Self::client_error(value)
//     }
// }

// impl From<TombCryptError> for SyncError {
//     fn from(value: TombCryptError) -> Self {
//         Self::client_error(ApiError::crypto(value))
//     }
// }
