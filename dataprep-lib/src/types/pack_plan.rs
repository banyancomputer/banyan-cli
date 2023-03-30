use crate::types::{
    shared::{CompressionScheme, EncryptionScheme, PartitionScheme},
    spider::SpiderMetadata,
};
use std::{path::PathBuf, sync::Arc};

/// this struct is used to build up the data processing steps for a file
#[derive(Debug, Clone)]
pub struct PackPlan {
    /// Describes how we will compress the file (contains compression algorithm info)
    pub compression: CompressionScheme,
    /// Describes how we will partition the file (contains partition algorithm info)
    pub partition: PartitionScheme,
    /// Describes how we will encrypt the file (contains keys)
    pub encryption: EncryptionScheme,
    /// file size in bytes before compression
    pub size_in_bytes: u128,
}
impl PackPlan {
    /// computes the number of chunks that will be produced by this packplan
    pub fn n_chunks(&self) -> u32 {
        // divide size_in_bytes by chunk_size, rounding up
        let n_chunks = self.size_in_bytes / self.partition.chunk_size as u128;
        if self.size_in_bytes % self.partition.chunk_size as u128 == 0 {
            n_chunks as u32
        } else {
            (n_chunks + 1) as u32
        }
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
    FileGroup(Vec<Arc<SpiderMetadata>>, PackPlan),
}
impl PackPipelinePlan {
    /// computes the number of chunks that will be produced by this packpipelineplan
    pub fn n_chunks(&self) -> u32 {
        match self {
            PackPipelinePlan::Directory(_) => 1,
            PackPipelinePlan::Symlink(_, _) => 1,
            PackPipelinePlan::FileGroup(_, plan) => plan.n_chunks(),
        }
    }
    /// computes the number of bytes that will be produced by this packpipelineplan
    pub fn n_bytes(&self) -> u128 {
        match self {
            PackPipelinePlan::Directory(_) => 0,
            PackPipelinePlan::Symlink(_, _) => 0,
            PackPipelinePlan::FileGroup(_, plan) => plan.size_in_bytes,
        }
    }
}
