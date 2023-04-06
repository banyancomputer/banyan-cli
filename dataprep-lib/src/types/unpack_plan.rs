use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use wnfs::{common::DiskBlockStore, libipld::Cid};

/// This is the struct that becomes the contents of the manifest file.
/// It may seem silly to have a struct that has only one field, but in
/// versioning this struct, we can also version its children identically.
/// As well as any other fields we may add / remove in the future.
#[derive(Debug, Serialize, Deserialize)]
pub struct ManifestData {
    /// The project version that was used to encode this ManifestData
    pub version: String,
    /// The BlockStore that holds all packed data
    pub store: DiskBlockStore,
    /// The store CID that points to the PrivateRef of the PrivateDirectory
    pub ref_cid: Cid,
    /// The store CID that points to the IPLD DAG representing the PrivateForest
    pub ipld_cid: Cid,
}
