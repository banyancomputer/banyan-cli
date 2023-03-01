use anyhow::Result;
use std::{
    collections::HashMap, 
    path::PathBuf
    rc::Rc, 
    sync::Arc
};
use uuid::Uuid;

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::types::plan::{
    CompressionPlan, DataProcessPlan, DuplicationMetadata, DuplicationPlan, EncryptionPlan,
    PartitionPlan, PipelinePlan, WriteoutPlan,

};

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
    // This data structure is a little uglier than i would like it to be
    seen_hashes: Arc<RwLock<HashMap<String, DuplicationMetadata>>>,
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
        // Determine whether or not the file is a duplicate based on this hash
        let duplicate_metadata = {
            // grab a read lock and check if we've seen it
            let seen_hashes = seen_hashes.read().await;
            seen_hashes.get(&file_hash).cloned()
        };
        // If there are file names associated with this hash, we've seen it before
        let file_is_duplicate = duplicate_metadata.is_some();

        // Enclose origin data in Reference Counter
        let od = Rc::new(origin_data);

        // Determine file size based on metadata
        let file_size = od.original_metadata.len();
        // Determine number of chunks required to store this file
        let num_chunks = (file_size as f64 / target_chunk_size as f64).ceil() as u64;

        // Compression and partition plans are the same even in the duplicate case
        let compression_plan = CompressionPlan::new_gzip();
        let partition_plan = PartitionPlan::new(target_chunk_size, num_chunks);

        // Create a DataProcessPlan based on duplication status
        let process_plan = if file_is_duplicate {
            // DataProcessPlan for a duplicate file
            DataProcessPlan {
                compression: compression_plan,
                partition: partition_plan,
                // The encryption key was already generated the first time this file was seen
                encryption: duplicate_metadata.clone().unwrap().key,
                writeout: WriteoutPlan {
                    // The output paths were already generated the first time this file was seen
                    output_paths: duplicate_metadata.unwrap().locations,
                },
                duplication: DuplicationPlan {
                    // This represents the location that the unpacker will extract the file to
                    expected_location: Some(od.original_location.to_path_buf()),
                },
            }
        } else {
            // DataProcessPlan for a new file
            DataProcessPlan {
                compression: compression_plan,
                partition: partition_plan,
                // Construct a new encryption key
                encryption: EncryptionPlan::new(),
                writeout: WriteoutPlan {
                    // Construct a vector of random filenames for writing
                    output_paths: (0..num_chunks)
                        .map(|_| Uuid::new_v4())
                        .map(|f| to_root.join(f.to_string()))
                        .collect::<Vec<PathBuf>>(),
                },
                // There is no duplication plan for new files
                duplication: DuplicationPlan::none(),
            }
        };

        // Create a PipelinePlan
        let pipeline_plan = PipelinePlan {
            origin_data: od.clone(),
            data_processing: DataProcessDirective::File(process_plan.clone()),
        };

        {
            // Grab write lock
            let mut seen_hashes = seen_hashes.write().await;
            // Insert the file hash and DuplicationMetadata into the HashMap
            seen_hashes.insert(file_hash.clone(), process_plan.try_into().unwrap());
        }

        // Now that plan is created and file is marked as seen, return Ok
        Ok(pipeline_plan)
    }
}

// TODO (thea-exe): Our inline tests
#[cfg(test)]
mod test {}
