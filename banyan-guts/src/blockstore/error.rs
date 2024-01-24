use colored::Colorize;
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use crate::{car::error::CarError, WnfsError};

#[derive(Debug)]
pub struct BlockStoreError {
    kind: BlockStoreErrorKind,
}

impl std::error::Error for BlockStoreError {}

impl BlockStoreError {
    pub fn missing_file(path: &Path) -> Self {
        Self {
            kind: BlockStoreErrorKind::MissingFile(path.to_path_buf()),
        }
    }

    pub fn missing_directory(path: &Path) -> Self {
        Self {
            kind: BlockStoreErrorKind::MissingDirectory(path.to_path_buf()),
        }
    }

    pub fn exists(path: &Path) -> Self {
        Self {
            kind: BlockStoreErrorKind::Exists(path.to_path_buf()),
        }
    }

    pub fn car(err: CarError) -> Self {
        Self {
            kind: BlockStoreErrorKind::Car(err),
        }
    }

    pub fn wnfs(err: WnfsError) -> Self {
        Self {
            kind: BlockStoreErrorKind::Wnfs(err),
        }
    }
}

impl Display for BlockStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match &self.kind {
            BlockStoreErrorKind::MissingFile(file) => {
                format!("Expected and failed to find file: {}", file.display())
            }
            BlockStoreErrorKind::MissingDirectory(dir) => {
                format!("Expected and failed to find directory: {}", dir.display())
            }
            BlockStoreErrorKind::Exists(dir) => {
                format!(
                    "Tried to create a BlockStore at a directory which was already populated: {}",
                    dir.display()
                )
            }
            BlockStoreErrorKind::Car(err) => format!("{} {err}", "CAR ERROR:".underline()),
            BlockStoreErrorKind::Wnfs(err) => format!("{} {err}", "WNFS ERROR:".underline()),
        };

        f.write_str(&string)
    }
}

#[derive(Debug)]
pub enum BlockStoreErrorKind {
    MissingFile(PathBuf),
    MissingDirectory(PathBuf),
    Exists(PathBuf),
    Car(CarError),
    Wnfs(WnfsError),
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

impl From<WnfsError> for BlockStoreError {
    fn from(value: WnfsError) -> Self {
        Self::wnfs(value)
    }
}

impl From<wnfs::libipld::cid::Error> for BlockStoreError {
    fn from(value: wnfs::libipld::cid::Error) -> Self {
        Self::car(CarError::cid_error(value))
    }
}
