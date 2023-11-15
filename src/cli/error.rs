// use std::fmt::Display;
// use thiserror::Error;

// use crate::{api::error::ApiError, native::NativeError, filesystem::FilesystemError};

// #[derive(Debug, Error)]
// pub(crate) struct CliError {
//     kind: CliErrorKind,
// }


// impl CliError {
//     pub(crate) fn native(err: NativeError) -> Self {
//         Self {
//             kind: CliErrorKind::Native(err),
//         }
//     }

//     pub(crate) fn api(err: ApiError) -> Self {
//         Self {
//             kind: CliErrorKind::Native(NativeError::api(err)),
//         }
//     }
// }

// #[derive(Debug)]
// pub(crate) enum CliErrorKind {
//     Native(NativeError),
// }

// impl From<NativeError> for CliError {
//     fn from(value: NativeError) -> Self {
//         Self::native(value)
//     }
// }

// impl From<ApiError> for CliError {
//     fn from(value: ApiError) -> Self {
//         Self::api(value)
//     }
// }

// impl From<FilesystemError> for CliError {
//     fn from(value: FilesystemError) -> Self {
//         Self::native(NativeError::filesytem(value))
//     }
// }