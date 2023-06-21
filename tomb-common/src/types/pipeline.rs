use super::blockstore::networkblockstore::NetworkBlockStore;
use crate::types::blockstore::carblockstore::CarBlockStore;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Debug};
use wnfs::libipld::Cid;

/// This is the struct that becomes the contents of the manifest file.
/// It may seem silly to have a struct that has only one field, but in
/// versioning this struct, we can also version its children identically.
/// As well as any other fields we may add / remove in the future.
#[derive(Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    /// The project version that was used to encode this Manifest
    pub version: String,
    /// The BlockStore that holds all packed data
    pub cold_local: CarBlockStore,
    /// The BlockStore that holds all packed data, remotely
    pub cold_remote: NetworkBlockStore,
    /// The BlockStore that holds all Metadata
    pub hot_local: CarBlockStore,
    /// The BlockStore that holds all Metadata, remotely
    pub hot_remote: NetworkBlockStore,
    /// The roots containing keys and CIDs
    pub roots: HashMap<String, Cid>,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            cold_local: Default::default(),
            cold_remote: Default::default(),
            hot_local: Default::default(),
            hot_remote: Default::default(),
            roots: Default::default(),
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
