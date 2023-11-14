use std::path::{Path, PathBuf};

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

pub enum BlockStoreErrorKind {
    NoSuchFile,
    ExpectedFile(PathBuf),
    ExpectedDirectory(PathBuf),
    Car(CarError),
}
