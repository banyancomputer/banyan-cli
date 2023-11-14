use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use anyhow::anyhow;

use crate::car::error::CarError;

#[derive(Debug)]
pub struct BlockStoreError {
    pub(crate) kind: BlockStoreErrorKind,
}

impl BlockStoreError {
    pub fn no_such_file() -> Self {
        Self {
            kind: BlockStoreErrorKind::NoSuchFile,
        }
    }

    pub fn car(err: CarError) -> Self {
        Self {
            kind: BlockStoreErrorKind::Car(err),
        }
    }

    pub fn expected_file(path: &Path) -> Self {
        Self {
            kind: BlockStoreErrorKind::ExpectedFile(path.to_path_buf()),
        }
    }

    pub fn expected_directory(path: &Path) -> Self {
        Self {
            kind: BlockStoreErrorKind::ExpectedDirectory(path.to_path_buf()),
        }
    }
}

impl Display for BlockStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Debug)]
pub enum BlockStoreErrorKind {
    NoSuchFile,
    ExpectedFile(PathBuf),
    ExpectedDirectory(PathBuf),
    Car(CarError),
}

impl From<CarError> for BlockStoreError {
    fn from(value: CarError) -> Self {
        Self::car(value)
    }
}

impl From<std::io::Error> for BlockStoreError {
    fn from(value: std::io::Error) -> Self {
        Self::car(CarError::io_error(value))
    }
}

impl From<anyhow::Error> for BlockStoreError {
    fn from(value: anyhow::Error) -> Self {
        todo!()
    }
}

impl From<wnfs::libipld::cid::Error> for BlockStoreError {
    fn from(value: wnfs::libipld::cid::Error) -> Self {
        Self::car(CarError::cid_error(value))
    }
}

impl From<BlockStoreError> for anyhow::Error {
    fn from(value: BlockStoreError) -> Self {
        anyhow!("blockstore error: {:?}", value)
    }
}
