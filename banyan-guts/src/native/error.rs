use std::{fmt::Display, string::FromUtf8Error};

use colored::Colorize;
use tomb_crypt::prelude::TombCryptError;

use crate::{
    api::error::ApiError, blockstore::BlockStoreError, car::error::CarError,
    filesystem::FilesystemError, WnfsError,
};

#[cfg(feature = "cli")]
use {crate::cli::specifiers::DriveSpecifier, std::path::PathBuf, uuid::Uuid};

#[derive(Debug)]
pub struct NativeError {
    kind: NativeErrorKind,
}

impl std::error::Error for NativeError {}

impl Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match &self.kind {
            NativeErrorKind::MissingApiKey => "Unable to find API Key".to_owned(),
            NativeErrorKind::MissingWrappingKey => "Unable to find Wrapping Key".to_owned(),
            NativeErrorKind::MissingUserId => "Unable to find remote User Id".to_owned(),
            NativeErrorKind::MissingIdentifier => {
                "Unable to find a remote Identifier associated with that Drive".to_owned()
            }
            NativeErrorKind::DaemonError(err) => {
                format!("{} {err}", "DAEMON ERROR:".underline())
            }
            NativeErrorKind::MissingLocalDrive => {
                "Unable to find a local Drive with that query".to_owned()
            }
            NativeErrorKind::MissingRemoteDrive => {
                "Unable to find a remote Drive with that query".to_owned()
            }
            NativeErrorKind::UniqueDriveError => {
                "There is already a unique Drive with these specs".to_owned()
            }
            NativeErrorKind::BadData => "bad data".to_owned(),
            NativeErrorKind::Custom(msg) => msg.to_owned(),
            NativeErrorKind::Cryptographic(err) => {
                format!("{} {err}", "CRYPTOGRAPHIC ERROR:".underline())
            }
            NativeErrorKind::Filesystem(err) => {
                format!("{} {err}", "FILESYSTEM ERROR:".underline())
            }
            NativeErrorKind::Api(err) => format!("{} {err}", "CLIENT ERROR:".underline()),
            NativeErrorKind::Io(err) => format!("{} {err}", "IO ERROR:".underline()),
            #[cfg(feature = "cli")]
            NativeErrorKind::UnknownDrive(_) => "No known Drive with that specification".to_owned(),
        };

        f.write_str(&string)
    }
}

impl NativeError {
    pub fn missing_api_key() -> Self {
        Self {
            kind: NativeErrorKind::MissingApiKey,
        }
    }

    pub fn missing_wrapping_key() -> Self {
        Self {
            kind: NativeErrorKind::MissingWrappingKey,
        }
    }

    pub fn missing_user_id() -> Self {
        Self {
            kind: NativeErrorKind::MissingUserId,
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

    pub fn io(err: std::io::Error) -> Self {
        Self {
            kind: NativeErrorKind::Io(err),
        }
    }

    /// Unknown Bucket path
    #[cfg(feature = "cli")]
    pub fn unknown_path(path: PathBuf) -> Self {
        Self {
            kind: NativeErrorKind::UnknownDrive(DriveSpecifier::with_origin(&path)),
        }
    }

    /// Unknown Bucket ID
    #[cfg(feature = "cli")]
    pub fn unknown_id(id: Uuid) -> Self {
        Self {
            kind: NativeErrorKind::UnknownDrive(DriveSpecifier::with_id(id)),
        }
    }

    /// Daemon errors
    #[cfg(feature = "cli")]
    pub fn daemon_error(msg: String) -> Self {
        Self {
            kind: NativeErrorKind::DaemonError(msg),
        }
    }
}

#[derive(Debug)]
enum NativeErrorKind {
    MissingApiKey,
    MissingWrappingKey,
    MissingUserId,
    MissingIdentifier,
    MissingLocalDrive,
    MissingRemoteDrive,
    UniqueDriveError,
    BadData,
    DaemonError(String),
    Custom(String),
    Cryptographic(TombCryptError),
    Filesystem(Box<FilesystemError>),
    Api(ApiError),
    Io(std::io::Error),
    #[cfg(feature = "cli")]
    UnknownDrive(DriveSpecifier),
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

impl From<WnfsError> for NativeError {
    fn from(value: WnfsError) -> Self {
        Self::filesytem(FilesystemError::wnfs(value))
    }
}

impl From<BlockStoreError> for NativeError {
    fn from(value: BlockStoreError) -> Self {
        Self::filesytem(FilesystemError::blockstore(value))
    }
}

impl From<std::io::Error> for NativeError {
    fn from(value: std::io::Error) -> Self {
        Self::io(value)
    }
}

impl From<FromUtf8Error> for NativeError {
    fn from(value: FromUtf8Error) -> Self {
        Self::custom_error(&format!("From UTF8: {value}"))
    }
}
