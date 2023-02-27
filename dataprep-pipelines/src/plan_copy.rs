use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::types::plan::{
    CompressionPlan, 
    DataProcessPlan, 
    EncryptionPlan, 
    PartitionPlan, 
    PipelinePlan, 
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
    seen_hashes: Arc<RwLock<HashMap<String, Rc<SpiderMetadata>>>>,
    target_chunk_size: u64,
) -> Result<PipelinePlan> {
    println!("current path: {}", origin_data.original_location.display());

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
        // Grab the associated value from the hashmap
        let maybe_duplicate_path = {
            // grab a read lock and check if we've seen it
            let seen_hashes = seen_hashes.read().await;
            seen_hashes.get(&file_hash).cloned()
        };

        // If the value exists, we've seen this file before
        if let Some(duplicate_path) = maybe_duplicate_path {
            // TODO remove; this is just for debugging
            println!(
                "I've seen this file before!\nog: {}\nnew: {}",
                duplicate_path.original_location.display(),
                origin_data.original_location.display()
            );

            // Point to the first file we saw with this hash. all done
            let od = Rc::new(origin_data);
            // This PilelinePlan captures both the data we are currently processing (od), and 
            // the duplicate path from which the hash was originally generated, now stored in data_processing
            // Missing here, though, is knowledge of the DataProcessPlan that was used to encode the original file.
            // To fully implement duplication, this will need to be added to the Duplicate DataProcessDirective in
            // one form or another.
            Ok(PipelinePlan {
                origin_data: od,
                data_processing: DataProcessDirective::Duplicate(
                    Rc::clone(&duplicate_path)
                ),
            })
        } 
        // If the value does not exist, this is a new file
        else {
            // make random filenames to put this thing's chunks in
            let od = Rc::new(origin_data);
            
            // drop the write lock

            // how long is the file?
            let file_size = od.original_metadata.len();
            // how many chunks will we need to make?
            let num_chunks = (file_size as f64 / target_chunk_size as f64).ceil() as u64;
            let random_filenames = (0..num_chunks)
                .map(|_| Uuid::new_v4())
                .map(|f| to_root.join(f.to_string()))
                .collect();

            // Create a DataProcessPlan
            let process_plan = DataProcessPlan {
                compression: CompressionPlan::new_gzip(),
                partition: PartitionPlan::new(target_chunk_size, num_chunks),
                encryption: EncryptionPlan::new(),
                writeout: WriteoutPlan {
                    output_paths: random_filenames,
                },
            };

            // Create a PipelinePlan
            let pipeline_plan = PipelinePlan {
                origin_data: od.clone(),
                data_processing: DataProcessDirective::File(process_plan),
            };

            // Add to hashmap using file-specific data so that duplicates know where to look
            {
                // otherwise, get the write lock and add this file's SpiderMetadata to the seen hashes
                let mut seen_hashes = seen_hashes.write().await;
                // Using the hash as key, insert the SpiderMetadata and DataProcessPlan
                seen_hashes.insert(file_hash.clone(), od);
            }

            // Return Ok with PipelinePlan
            Ok(pipeline_plan)
        }
    }
}

// TODO (thea-exe): Our inline tests
#[cfg(test)]
mod test {}
