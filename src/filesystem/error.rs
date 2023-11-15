use std::fmt::Display;

use colored::Colorize;
use tomb_crypt::prelude::TombCryptError;

use crate::blockstore::BlockStoreError;

use super::sharing::SharingError;

#[derive(Debug)]
pub struct FilesystemError {
    kind: FilesystemErrorKind,
}

impl Display for FilesystemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match &self.kind {
            FilesystemErrorKind::MissingMetadata(label) => {
                format!("Missing metadata with label \"{label}\"")
            }
            FilesystemErrorKind::NodeNotFound(path) => {
                format!("Unable to find node with path \"{path}\"")
            }
            FilesystemErrorKind::Sharing(err) => format!("{} {err}", "SHARING ERROR:".underline()),
            FilesystemErrorKind::Blockstore(err) => {
                format!("{} {err}", "BLOCKSTORE ERROR:".underline())
            }
            FilesystemErrorKind::Wnfs(err) => format!("{} {err}", "WNFS ERROR:".underline()),
        };

        f.write_str(&string)
    }
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
}

#[derive(Debug)]
pub enum FilesystemErrorKind {
    MissingMetadata(String),
    NodeNotFound(String),
    Sharing(SharingError),
    Blockstore(BlockStoreError),
    Wnfs(anyhow::Error),
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
