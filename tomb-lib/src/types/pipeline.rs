use crate::types::{blockstore::carblockstore::CarBlockStore, spider::SpiderMetadata};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::PathBuf, sync::Arc};

/// This is the struct that becomes the contents of the manifest file.
/// It may seem silly to have a struct that has only one field, but in
/// versioning this struct, we can also version its children identically.
/// As well as any other fields we may add / remove in the future.
#[derive(Serialize, Deserialize)]
pub struct Manifest {
    /// The project version that was used to encode this Manifest
    pub version: String,
    /// The BlockStore that holds all packed data
    pub content_store: CarBlockStore,
    /// The BlockStore that holds all Metadata
    pub meta_store: CarBlockStore,
}

impl Debug for Manifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Manifest")
            .field("version", &self.version)
            .finish()
    }
}

/// This struct is used to describe how a filesystem structure was processed. Either it was a duplicate/symlink/
/// directory and there isn't much to do, or else we need to go through compression, partition, and
/// encryption steps.
/// this takes in pre-grouped files (for processing together) or marked directories/simlinks.
#[derive(Debug, Clone)]
pub enum PackPipelinePlan {
    /// It was a directory, just create it
    Directory(Arc<SpiderMetadata>),
    /// it was a symlink, just create it (with destination)
    Symlink(Arc<SpiderMetadata>, PathBuf),
    /// it was a group of identical files, here's the metadata for how they were encrypted and compressed
    FileGroup(Vec<Arc<SpiderMetadata>>),
}
