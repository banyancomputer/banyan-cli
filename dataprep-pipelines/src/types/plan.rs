use crate::types::pipeline::PartitionMetadata;
use crate::types::shared::DataProcessDirective;
use crate::types::spider::SpiderMetadata;
use std::path::PathBuf;
use std::rc::Rc;

// TODO (laudiacay) continue making types better...

#[derive(Debug, Clone)]
pub struct CompressionPlan {
    pub compression_info: String,
}
impl CompressionPlan {
    pub fn new_gzip() -> Self {
        CompressionPlan {
            compression_info: "GZIP".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PartitionPlan(pub PartitionMetadata);

impl PartitionPlan {
    pub fn new(chunk_size: u64, num_chunks: u64) -> Self {
        PartitionPlan(PartitionMetadata {
            chunk_size,
            num_chunks,
        })
    }
}

#[derive(Clone)]
pub struct EncryptionPlan {
    pub identity: age::x25519::Identity,
}

impl EncryptionPlan {
    pub fn new() -> Self {
        EncryptionPlan {
            identity: age::x25519::Identity::generate(),
        }
    }
}

impl Default for EncryptionPlan {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct WriteoutPlan {
    pub output_paths: Vec<PathBuf>,
}

/// this struct is used to build up the data processing steps for a file
#[derive(Clone)]
pub struct DataProcessPlan {
    /// describes how we will compress the file
    pub compression: CompressionPlan,
    /// describes how we will partition the file
    pub partition: PartitionPlan,
    /// describes how we will encrypt the file
    pub encryption: EncryptionPlan,
    /// describes how we will write the file out on the new filesystem
    pub writeout: WriteoutPlan,
}

pub struct PipelinePlan {
    /// describes where a file came from on the original filesystem
    pub origin_data: Rc<SpiderMetadata>,
    /// describes data processing, if any is needed
    pub data_processing: DataProcessDirective<DataProcessPlan>,
}
