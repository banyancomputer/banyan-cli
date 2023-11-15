use tomb_crypt::prelude::TombCryptError;

use crate::blockstore::BlockStoreError;

use super::sharing::SharingError;

#[derive(Debug)]
pub struct FilesystemError {
    pub kind: FilesystemErrorKind,
}

impl FilesystemError {
    pub fn node_not_found(path: &str) -> Self {
        Self {
            kind: FilesystemErrorKind::NodeNotFound(path.to_string()),
        }
    }

    pub fn missing_metadata(label: &str) -> Self {
        Self {
            kind: FilesystemErrorKind::MissingMetadata(label.to_string()),
        }
    }

    pub fn sharing(err: SharingError) -> Self {
        Self {
            kind: FilesystemErrorKind::Sharing(err),
        }
    }

    pub fn blockstore(err: BlockStoreError) -> Self {
        Self {
            kind: FilesystemErrorKind::Blockstore(err),
        }
    }

    pub fn wnfs(err: anyhow::Error) -> Self {
        Self {
            kind: FilesystemErrorKind::Wnfs(err),
        }
    }

    pub fn io(err: std::io::Error) -> Self {
        Self {
            kind: FilesystemErrorKind::Io(err),
        }
    }
}

#[derive(Debug)]
pub enum FilesystemErrorKind {
    MissingMetadata(String),
    NodeNotFound(String),
    BadConfig,
    Sharing(SharingError),
    Blockstore(BlockStoreError),
    Wnfs(anyhow::Error),
    Io(std::io::Error),
}

impl From<SharingError> for FilesystemError {
    fn from(value: SharingError) -> Self {
        Self::sharing(value)
    }
}

impl From<TombCryptError> for FilesystemError {
    fn from(value: TombCryptError) -> Self {
        Self::sharing(SharingError::cryptographic(value))
    }
}

impl From<BlockStoreError> for FilesystemError {
    fn from(value: BlockStoreError) -> Self {
        Self::blockstore(value)
    }
}

impl From<anyhow::Error> for FilesystemError {
    fn from(value: anyhow::Error) -> Self {
        Self::wnfs(value)
    }
}

impl From<std::io::Error> for FilesystemError {
    fn from(value: std::io::Error) -> Self {
        Self::io(value)
    }
}
