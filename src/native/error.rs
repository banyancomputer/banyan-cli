use tomb_crypt::prelude::TombCryptError;

use crate::{
    api::error::ApiError, blockstore::BlockStoreError, car::error::CarError,
    filesystem::FilesystemError,
};

#[cfg(feature = "cli")]
use {crate::cli::specifiers::DriveSpecifier, std::path::PathBuf, uuid::Uuid};

#[derive(Debug)]
pub(crate) struct NativeError {
    kind: NativeErrorKind,
}

impl NativeError {
    pub(crate) fn missing_credentials() -> Self {
        Self {
            kind: NativeErrorKind::MissingCredentials,
        }
    }

    pub(crate) fn missing_identifier() -> Self {
        Self {
            kind: NativeErrorKind::MissingIdentifier,
        }
    }

    pub(crate) fn missing_local_drive() -> Self {
        Self {
            kind: NativeErrorKind::MissingLocalDrive,
        }
    }

    pub(crate) fn missing_remote_drive() -> Self {
        Self {
            kind: NativeErrorKind::MissingRemoteDrive,
        }
    }

    pub(crate) fn unique_error() -> Self {
        Self {
            kind: NativeErrorKind::UniqueDriveError,
        }
    }

    pub(crate) fn bad_data() -> Self {
        Self {
            kind: NativeErrorKind::BadData,
        }
    }

    pub(crate) fn custom_error(msg: &str) -> Self {
        Self {
            kind: NativeErrorKind::Custom(msg.to_owned()),
        }
    }

    pub(crate) fn cryptographic(err: TombCryptError) -> Self {
        Self {
            kind: NativeErrorKind::Cryptographic(err),
        }
    }

    pub(crate) fn filesytem(err: FilesystemError) -> Self {
        Self {
            kind: NativeErrorKind::Filesystem(err),
        }
    }

    pub(crate) fn api(err: ApiError) -> Self {
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
    Filesystem(FilesystemError),
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
