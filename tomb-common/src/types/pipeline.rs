use crate::types::blockstore::carblockstore::CarBlockStore;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use super::blockstore::networkblockstore::NetworkBlockStore;

/// This is the struct that becomes the contents of the manifest file.
/// It may seem silly to have a struct that has only one field, but in
/// versioning this struct, we can also version its children identically.
/// As well as any other fields we may add / remove in the future.
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    /// The project version that was used to encode this Manifest
    pub version: String,
    /// The BlockStore that holds all packed data
    pub content_local: CarBlockStore,
    /// The BlockStore that holds all packed data, remotely
    pub content_remote: NetworkBlockStore,
    /// The BlockStore that holds all Metadata
    pub meta_store: CarBlockStore,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            content_local: Default::default(),
            content_remote: Default::default(),
            meta_store: Default::default(),
        }
    }
}

impl Debug for Manifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Manifest")
            .field("version", &self.version)
            .finish()
    }
}
