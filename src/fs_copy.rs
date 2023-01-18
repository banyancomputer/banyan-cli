use anyhow::Result;
use std::collections::HashMap;

use jwalk::DirEntry;
use std::fs::Metadata;
use std::path::PathBuf;

use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;

use crate::hasher;

// How large a buffer to use for copying files
const BUF_SIZE: usize = 1024 * 1024; // 1MB

#[derive(Debug)]
/// Enum that describes whether this file is a duplicate or not
pub enum DuplicateOrOriginal {
    Duplicate(PathBuf),
    Original,
}

/// Implements test for DuplicateOrOriginal
impl DuplicateOrOriginal {
    /// Returns true if this is a duplicate
    fn is_original(&self) -> bool {
        match self {
            DuplicateOrOriginal::Duplicate(_) => false,
            DuplicateOrOriginal::Original => true,
        }
    }
}

#[derive(Debug)]
/// MetaData that is emitted on successful copy
pub struct CopyMetadata {
    /// Original root path of the file
    pub(crate) original_root: PathBuf,
    /// Original path of the file within the root
    pub(crate) original_location: DirEntry<(u64, Option<u64>)>,
    /// Original metadata of the file
    pub(crate) original_metadata: Metadata,
    /// New location of the file
    pub(crate) new_location: PathBuf,
    /// the blake2 hash of the file
    pub(crate) blake2_file_hash: Option<String>,
    /// Whether or not this file or directory is a duplicate
    pub(crate) duplicate_or_original: DuplicateOrOriginal,
}

/// Copy a file or directory from one location to another. If the file is a duplicate, it will not be copied.
/// This is meant to be run in a tokio task, so be sure to use tokio::fs functions!
///
/// # Arguments
/// original_root: The root path of the original file or directory
/// original_location: The path of the original file or directory within the root
/// to_root: The root path to which the file or directory will be copied
/// seen_hashes: A hashmap of blake2 hashes. Used to determine if a file is a duplicate or not.
///
/// # Returns
/// CopyMetadata struct that contains the original and new location of the file, as well as the blake2 hash of the file.
pub async fn copy_file_or_dir(
    original_root: PathBuf,
    original_location: DirEntry<(u64, Option<u64>)>,
    to_root: PathBuf,
    seen_hashes: Arc<RwLock<HashMap<String, PathBuf>>>,
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
            new_location: new_path,
            blake2_file_hash: None,
            duplicate_or_original: DuplicateOrOriginal::Original,
        })
    }
    // Otherwise if this is a symlink
    // TODO (laudiacay): Handle symlinks
    else if original_metadata.is_symlink() {
        // Note (laudiacay) Reading symlinks from the filesystem looks annoying. Coming back to this later.
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
            let mut seen_hashes = seen_hashes.write().await;
            // If we've seen this file before,
            if let Some(duplicate_path) = seen_hashes.get(&file_hash) {
                // Point to the first file we saw with this hash
                DuplicateOrOriginal::Duplicate(duplicate_path.clone())
            } else {
                // otherwise, add this file to the seen hashes
                seen_hashes.insert(file_hash.clone(), new_path.clone());
                // and return that this is the original
                DuplicateOrOriginal::Original
            }
        };
        // If this is not a duplicate,
        if duplicate_or_original.is_original() {
            // Open the original file
            let mut original_file = tokio::fs::File::open(&original_location.path()).await?;
            // Create the new file at the new location
            let mut new_file = tokio::fs::File::create(&new_path).await?;
            // Allocate a buffer to use for copying
            let mut buf = [0u8; BUF_SIZE];
            // Read data from the original file into the buffer and into the new file
            loop {
                let n = original_file.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                let n2 = new_file.write(&buf[..n]).await?;
                assert_eq!(n, n2);
            }
            {
                // lock guards
                let mut seen_hashes = seen_hashes.write().await;
                seen_hashes.insert(file_hash.clone(), new_path.clone());
            }
        }

        // Return the CopyMetadata struct
        Ok(CopyMetadata {
            original_root,
            original_location,
            original_metadata,
            new_location: new_path,
            duplicate_or_original,
            blake2_file_hash: Some(file_hash),
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
