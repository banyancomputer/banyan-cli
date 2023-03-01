use serde::{Deserialize, Serialize};

use crate::types::{
    pipeline::PartitionMetadata, shared::DataProcessDirective, spider::SpiderMetadata,
};
use std::{path::PathBuf, sync::Arc};

// TODO (laudiacay) continue making types better...

#[derive(Debug, Clone, Serialize)]
pub struct CompressionPlan {
    pub compression_info: String,
}
impl CompressionPlan {
    pub fn new_zstd() -> Self {
        CompressionPlan {
            compression_info: "ZSTD".to_string(),
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

impl std::fmt::Debug for EncryptionPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "encryption plans are secret for now")
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteoutPlan {
    pub output_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicationPlan {
    pub expected_location: Option<PathBuf>,
}

impl DuplicationPlan {
    pub fn none() -> Self {
        DuplicationPlan {
            expected_location: None,
        }
    }
}

/// This struct is used to keep track of information that needs to be present in
/// both the original and duplicate versions of a file's DataProcessPlan. Particularly,
/// encryption and writeout locations must be present or the unpacker will not know
/// where to find the file or how to decrypt it.
#[derive(Debug, Clone)]
pub struct DuplicationMetadata {
    pub key: EncryptionPlan,
    pub locations: Vec<PathBuf>,
}

impl From<DataProcessPlan> for DuplicationMetadata {
    fn from(plan: DataProcessPlan) -> Self {
        let key = plan.encryption;
        let locations = plan.writeout.output_paths;

        DuplicationMetadata { key, locations }
    }
}

/// this struct is used to build up the data processing steps for a file
#[derive(Debug, Clone)]
pub struct DataProcessPlan {
    // Describes how we will compress the file
    pub compression: CompressionPlan,
    // Describes how we will partition the file
    pub partition: PartitionPlan,
    // Describes how we will encrypt the file
    pub encryption: EncryptionPlan,
    // Describes how we will write the file out on the new filesystem
    pub writeout: WriteoutPlan,
    // Described if/how we will deduplicate the file
    pub duplication: DuplicationPlan,
}

#[derive(Clone, Debug)]
pub struct PipelinePlan {
    /// describes where a file came from on the original filesystem
    pub origin_data: Arc<SpiderMetadata>,
    /// describes data processing, if any is needed
    pub data_processing: DataProcessDirective<DataProcessPlan>,
}
