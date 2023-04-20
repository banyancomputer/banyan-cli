use crate::types::spider::SpiderMetadata;
use serde::{Deserialize, Serialize};
use skip_ratchet::Ratchet;
use std::{fmt::Debug, path::PathBuf, sync::Arc};
use wnfs::{common::CarBlockStore, libipld::Cid};

/// This is the struct that becomes the contents of the manifest file.
/// It may seem silly to have a struct that has only one field, but in
/// versioning this struct, we can also version its children identically.
/// As well as any other fields we may add / remove in the future.
#[derive(Serialize, Deserialize)]
pub struct ManifestData {
    /// The project version that was used to encode this ManifestData
    pub version: String,
    /// The ratchet of the original Filesystem, the first time it is packed
    pub original_ratchet: Ratchet,
    /// The BlockStore that holds all packed data
    pub content_store: CarBlockStore,
    /// The BlockStore that holds all Metadata
    pub meta_store: CarBlockStore,
    /// The store CID that points to the PrivateRef of the PrivateDirectory
    pub ref_cid: Cid,
    /// The store CID that points to the IPLD DAG representing the PrivateForest
    pub ipld_cid: Cid,
}

impl Debug for ManifestData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManifestData")
            .field("version", &self.version)
            .field("ref_cid", &self.ref_cid)
            .field("ipld_cid", &self.ipld_cid)
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
