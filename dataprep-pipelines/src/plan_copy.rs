use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::types::plan::{
    CompressionPlan, DataProcessPlan, DuplicationPlan, EncryptionPlan, PartitionPlan, PipelinePlan,
    WriteoutPlan,
};
use crate::types::shared::DataProcessDirective;
use crate::types::spider::SpiderMetadata;
use crate::utils::hasher;

/// Copy a file or directory from one location to another. If the file is a duplicate, it will not be copied.
///
/// # Arguments
/// original_root: The root path of the original file or directory
/// original_location: The path of the original file or directory within the root
/// to_root: The root path to which the file or directory will be copied
/// seen_hashes: A hashmap of blake2 hashes. Used to determine if a file is a duplicate or not.
///
/// # Returns
/// CopyMetadata struct that contains the original and new location of the file, as well as the blake2 hash of the file.
// TODO (laudiacay): one day, do we use Rabin partitioning?
pub async fn plan_copy(
    origin_data: SpiderMetadata,
    to_root: PathBuf,
    seen_hashes: Arc<RwLock<HashSet<String>>>,
    target_chunk_size: u64,
) -> Result<PipelinePlan> {
    // If this is a directory
    if origin_data.original_metadata.is_dir() {
        // Return
        Ok(PipelinePlan {
            origin_data: Rc::new(origin_data),
            data_processing: DataProcessDirective::Directory,
        })
    }
    // If this is a symlink
    else if origin_data.original_metadata.is_symlink() {
        // return
        Ok(PipelinePlan {
            origin_data: Rc::new(origin_data),
            data_processing: DataProcessDirective::Symlink,
        })
    }
    // If this is a file
    else {
        // Compute the file hash
        let file_hash = hasher::hash_file(&origin_data.canonicalized_path).await?;
        // Determine whether or not the file is a duplicate
        let file_is_duplicate = {
            // grab a read lock and check if we've seen it
            let seen_hashes = seen_hashes.read().await;
            seen_hashes.get(&file_hash).is_some()
        };

        // Enclose origin data in Reference Counter
        let od = Rc::new(origin_data);

        // Determine file size based on metadata
        let file_size = od.original_metadata.len();
        // Determine number of chunks required to store this file
        let num_chunks = (file_size as f64 / target_chunk_size as f64).ceil() as u64;
        // Generate a random filenames for each chunk, collected in a Vec
        let random_filenames = (0..num_chunks)
            .map(|_| Uuid::new_v4())
            .map(|f| to_root.join(f.to_string()))
            .collect();

        // Create a DataProcessPlan for packing and unpacking
        let process_plan = DataProcessPlan {
            compression: CompressionPlan::new_gzip(),
            partition: PartitionPlan::new(target_chunk_size, num_chunks),
            encryption: EncryptionPlan::new(),
            writeout: WriteoutPlan {
                output_paths: random_filenames,
            },
            duplication: DuplicationPlan {
                expected_location: if file_is_duplicate {
                    Some(od.original_location.to_path_buf())
                } else {
                    None
                },
            },
        };

        // Create a PipelinePlan
        let pipeline_plan = PipelinePlan {
            origin_data: od.clone(),
            data_processing: DataProcessDirective::File(process_plan),
        };

        {
            // Grab write lock
            let mut seen_hashes = seen_hashes.write().await;
            // Insert the file hash into the Hashset
            seen_hashes.insert(file_hash.clone());
        }

        // Now that plan is created and file is marked as seen, return Ok
        Ok(pipeline_plan)
    }
}

// TODO (thea-exe): Our inline tests
#[cfg(test)]
mod test {}
