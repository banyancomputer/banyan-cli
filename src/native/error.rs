use std::fmt::Display;

use tomb_crypt::prelude::TombCryptError;

use crate::{
    api::error::ApiError, blockstore::BlockStoreError, car::error::CarError,
    filesystem::FilesystemError,
};

#[cfg(feature = "cli")]
use {crate::cli::specifiers::DriveSpecifier, std::path::PathBuf, uuid::Uuid};

#[derive(Debug, thiserror::Error)]
pub struct NativeError {
    kind: NativeErrorKind,
}

impl Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl NativeError {
    pub fn missing_credentials() -> Self {
        Self {
            kind: NativeErrorKind::MissingCredentials,
        }
    }

    pub fn missing_identifier() -> Self {
        Self {
            kind: NativeErrorKind::MissingIdentifier,
        }
    }

    pub fn missing_local_drive() -> Self {
        Self {
            kind: NativeErrorKind::MissingLocalDrive,
        }
    }

    pub fn missing_remote_drive() -> Self {
        Self {
            kind: NativeErrorKind::MissingRemoteDrive,
        }
    }

    pub fn unique_error() -> Self {
        Self {
            kind: NativeErrorKind::UniqueDriveError,
        }
    }

    pub fn bad_data() -> Self {
        Self {
            kind: NativeErrorKind::BadData,
        }
    }

    pub fn custom_error(msg: &str) -> Self {
        Self {
            kind: NativeErrorKind::Custom(msg.to_owned()),
        }
    }

    pub fn cryptographic(err: TombCryptError) -> Self {
        Self {
            kind: NativeErrorKind::Cryptographic(err),
        }
    }

    pub fn filesytem(err: FilesystemError) -> Self {
        Self {
            kind: NativeErrorKind::Filesystem(Box::new(err)),
        }
    }

    pub fn api(err: ApiError) -> Self {
        Self {
            kind: NativeErrorKind::Api(err),
        }
    }

    /// Unknown Bucket path
    #[cfg(feature = "cli")]
    pub fn unknown_path(path: PathBuf) -> Self {
        Self {
            kind: NativeErrorKind::UnknownBucket(DriveSpecifier::with_origin(&path)),
        }
    }

    /// Unknown Bucket ID
    #[cfg(feature = "cli")]
    pub fn unknown_id(id: Uuid) -> Self {
        Self {
            kind: NativeErrorKind::UnknownBucket(DriveSpecifier::with_id(id)),
        }
    }
}

#[derive(Debug)]
enum NativeErrorKind {
    MissingCredentials,
    MissingIdentifier,
    MissingLocalDrive,
    MissingRemoteDrive,
    UniqueDriveError,
    BadData,
    Custom(String),
    Cryptographic(TombCryptError),
    Filesystem(Box<FilesystemError>),
    Api(ApiError),

    #[cfg(feature = "cli")]
    UnknownBucket(DriveSpecifier),
}

impl From<FilesystemError> for NativeError {
    fn from(value: FilesystemError) -> Self {
        Self::filesytem(value)
    }
}

impl From<CarError> for NativeError {
    fn from(value: CarError) -> Self {
        Self::filesytem(FilesystemError::blockstore(BlockStoreError::car(value)))
    }
}

impl From<TombCryptError> for NativeError {
    fn from(value: TombCryptError) -> Self {
        Self::cryptographic(value)
    }
}

impl From<ApiError> for NativeError {
    fn from(value: ApiError) -> Self {
        Self::api(value)
    }
}

impl From<anyhow::Error> for NativeError {
    fn from(value: anyhow::Error) -> Self {
        Self::filesytem(FilesystemError::wnfs(value))
    }
}

impl From<std::io::Error> for NativeError {
    fn from(value: std::io::Error) -> Self {
        Self::filesytem(FilesystemError::io(value))
    }
}

impl From<BlockStoreError> for NativeError {
    fn from(value: BlockStoreError) -> Self {
        Self::filesytem(FilesystemError::blockstore(value))
    }
}
