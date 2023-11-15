use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use colored::Colorize;

use crate::car::error::CarError;

#[derive(Debug)]
pub struct BlockStoreError {
    kind: BlockStoreErrorKind,
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

    pub fn wnfs(err: anyhow::Error) -> Self {
        Self {
            kind: BlockStoreErrorKind::Wnfs(err),
        }
    }
}

impl Display for BlockStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match &self.kind {
            BlockStoreErrorKind::NoSuchFile => "No such file".to_owned(),
            BlockStoreErrorKind::ExpectedFile(file) => {
                format!("Expected and failed to find file: {}", file.display())
            }
            BlockStoreErrorKind::ExpectedDirectory(dir) => {
                format!("Expected and failed to find directory: {}", dir.display())
            }
            BlockStoreErrorKind::Car(err) => format!("{} {err}", "CAR ERROR:".underline()),
            BlockStoreErrorKind::Wnfs(err) => format!("{} {err}", "WNFS ERROR:".underline()),
        };

        f.write_str(&string)
    }
}

#[derive(Debug)]
pub enum BlockStoreErrorKind {
    NoSuchFile,
    ExpectedFile(PathBuf),
    ExpectedDirectory(PathBuf),
    Car(CarError),
    Wnfs(anyhow::Error),
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
        Self::wnfs(value)
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
