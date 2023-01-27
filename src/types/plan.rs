use crate::types::pipeline::PartitionMetadata;
use crate::types::shared::DataProcessDirective;
use crate::types::spider::SpiderMetadata;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct CompressionPlan {
    pub(crate) compression_info: String,
}
impl CompressionPlan {
    pub(crate) fn new_gzip() -> Self {
        CompressionPlan {
            compression_info: "GZIP".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PartitionPlan(pub(crate) PartitionMetadata);

impl PartitionPlan {
    pub fn new(chunk_size: u64, num_chunks: u64) -> Self {
        PartitionPlan(PartitionMetadata { chunk_size , num_chunks})
    }
}

#[derive(Debug, Clone)]
pub struct EncryptionPlan {
    pub(crate) cipher_info: String,
    // TODO yikes
    pub(crate) tag_size: u64,
}

impl EncryptionPlan {
    pub fn new_aes_256_gcm() -> Self {
        EncryptionPlan {
            cipher_info: "AES-256-GCM".to_string(),
            tag_size: 16, // TODO is this right? CHECK
        }
    }
}

#[derive(Debug, Clone)]
pub struct WriteoutPlan {
    pub(crate) output_dir: PathBuf,
}

/// this struct is used to build up the data processing steps for a file
#[derive(Debug, Clone)]
pub struct DataProcessPlan {
    /// describes how we will compress the file
    pub(crate) compression: CompressionPlan,
    /// describes how we will partition the file
    pub(crate) partition: PartitionPlan,
    /// describes how we will encrypt the file
    pub(crate) encryption: EncryptionPlan,
    /// describes how we will write the file out on the new filesystem
    pub(crate) writeout: WriteoutPlan,
}

pub struct PipelinePlan {
    /// describes where a file came from on the original filesystem
    pub(crate) origin_data: Rc<SpiderMetadata>,
    /// describes data processing, if any is needed
    pub(crate) data_processing: DataProcessDirective<DataProcessPlan>,
}
