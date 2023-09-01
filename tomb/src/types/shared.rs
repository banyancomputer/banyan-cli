use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Wrapper for partitioning information
pub struct PartitionScheme {
    /// Maximum packing chunk size
    pub chunk_size: u64,
}
