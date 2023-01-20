use anyhow::Result;
use std::collections::HashMap;

use jwalk::DirEntry;
use std::fs::Metadata;
use std::path::PathBuf;

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{fsutil, hasher};

#[derive(Debug)]
/// MetaData that is emitted on successful copy
pub struct CopyMetadata {
    /// Original root path of the file
    pub(crate) original_root: PathBuf,
    /// Original path of the file within the root
    pub(crate) original_location: DirEntry<(u64, Option<u64>)>,
    /// Original metadata of the file
    pub(crate) original_metadata: Metadata,
    /// Whether or not this file or directory is a duplicate, and its location or the location of the original
    pub(crate) duplicate_or_original: fsutil::DuplicateOrOriginal,
    /// The partition guidelines for this file, if the file needs to be partitioned
    pub(crate) partition_guidelines: Option<fsutil::PartitionGuidelines>,
}

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
pub async fn prep_for_copy(
    original_root: PathBuf,
    original_location: DirEntry<(u64, Option<u64>)>,
    to_root: PathBuf,
    seen_hashes: Arc<RwLock<HashMap<String, PathBuf>>>,
    target_chunk_size: u64,
) -> Result<CopyMetadata> {
    // Determine the new location of the file
    let new_path = to_root.join(original_location.path().strip_prefix(&original_root)?);
    // Extract the metadata of the original file
    let original_metadata = original_location.metadata()?;
    // If this is a directory,
    if original_metadata.is_dir() {
        // Create the directory - if it already exists, this will do nothing
        tokio::fs::create_dir_all(&new_path).await?;
        // Return the CopyMetadata struct
        Ok(CopyMetadata {
            original_root,
            original_location,
            original_metadata: original_metadata.clone(),
            duplicate_or_original: fsutil::DuplicateOrOriginal::Original("".to_string()),
            partition_guidelines: None,
        })
    }
    // Otherwise if this is a symlink
    // TODO (laudiacay): Handle symlinks
    else if original_metadata.is_symlink() {
        // TODO (laudiacay) Reading symlinks from the filesystem looks annoying. Coming back to this later.
        // When we DO do this, we'll need to touch the destination file if it hasn't been passed over yet, then set the symlink
        // then fill in the destination file later as this iterator continues
        todo!("Handle symlinks");
    }
    // Otherwise this is just a file
    else {
        // Create the parent directory - if it already exists, this will do nothing
        tokio::fs::create_dir_all(new_path.parent().unwrap()).await?;
        // Compute the file hash
        let file_hash = hasher::hash_file(&original_location.path()).await?;
        // Check if we've seen this file before
        let duplicate_or_original = {
            let maybe_duplicate_path = {
                // get a read lock and check it
                let seen_hashes = seen_hashes.read().await;
                seen_hashes.get(&file_hash).cloned()
            };
            // If we've seen this file before,
            if let Some(duplicate_path) = maybe_duplicate_path {
                // Point to the first file we saw with this hash
                fsutil::DuplicateOrOriginal::Duplicate(file_hash, duplicate_path)
            } else {
                // otherwise, get the write lock and add this file to the seen hashes
                let mut seen_hashes = seen_hashes.write().await;
                seen_hashes.insert(file_hash.clone(), new_path.clone());
                // and return that this is the original
                fsutil::DuplicateOrOriginal::Original(file_hash)
            }
        };
        // time to figure out if we need to partition
        let file_size = original_metadata.len();
        let partition_guidelines =
            fsutil::make_partition(file_size, target_chunk_size, new_path.clone());

        // Return the CopyMetadata struct
        Ok(CopyMetadata {
            original_root,
            original_location,
            original_metadata,
            duplicate_or_original,
            partition_guidelines,
        })
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
