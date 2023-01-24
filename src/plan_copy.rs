use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::crypto_tools::hasher;
use crate::types::{
    CompressionPlan, DataProcessDirective, DataProcessPlan, EncryptionPlan, PartitionPlan,
    PipelinePlan, SpiderMetadata, WriteoutPlan,
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
    seen_hashes: Arc<RwLock<HashMap<String, Rc<SpiderMetadata>>>>,
    target_chunk_size: u64,
) -> Result<PipelinePlan> {
    // If this is a directory,
    if origin_data.original_metadata.is_dir() {
        // Return
        Ok(PipelinePlan {
            origin_data: Rc::new(origin_data),
            data_processing: DataProcessDirective::Directory,
        })
    } else if origin_data.original_metadata.is_symlink() {
        // return
        Ok(PipelinePlan {
            origin_data: Rc::new(origin_data),
            data_processing: DataProcessDirective::Symlink,
        })
    }
    // Otherwise this is just a file
    else {
        // Compute the file hash
        let file_hash = hasher::hash_file(&origin_data.canonicalized_path).await?;
        // Check if we've seen this file before
        let maybe_duplicate_path = {
            // grab a read lock and check if we've seen it
            let seen_hashes = seen_hashes.read().await;
            seen_hashes.get(&file_hash).cloned()
        };
        // If we've seen this file before,
        if let Some(duplicate_path) = maybe_duplicate_path {
            // Point to the first file we saw with this hash. all done
            let od = Rc::new(origin_data);
            Ok(PipelinePlan {
                origin_data: od,
                data_processing: DataProcessDirective::Duplicate(Rc::clone(&duplicate_path)),
            })
        } else {
            let random_filename = Uuid::new_v4();
            let _new_path = to_root.join(random_filename.to_string());
            let od = Rc::new(origin_data);

            {
                // otherwise, get the write lock and add this file's SpiderMetadata to the seen hashes
                let mut seen_hashes = seen_hashes.write().await;
                seen_hashes.insert(file_hash.clone(), od.clone());
            } // drop the write lock

            // make a random filename to put this in

            // Return the CopyMetadata struct
            Ok(PipelinePlan {
                origin_data: od,
                data_processing: DataProcessDirective::File(DataProcessPlan {
                    compression: CompressionPlan::new_gzip(),
                    partition: PartitionPlan::new_from_chunk_size(target_chunk_size),
                    encryption: EncryptionPlan::new_aes_256_gcm(),
                    writeout: WriteoutPlan {
                        output_dir: to_root.join(random_filename.to_string()),
                    },
                }),
            })
        }
    }
}

// TODO (xBalbinus & thea-exe): Our inline tests
#[cfg(test)]
mod test {
    // Note (amiller68): I'm pretty sure this needs to run in a tokio task, but I could be wrong.
    #[tokio::test]
    async fn test_copy_file_or_dir() {
        todo!("Write tests");
    }
}
