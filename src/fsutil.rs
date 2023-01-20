use crate::encryption_writer::EncryptionWriter;
use crate::fs_compression_encryption::{EncryptionMetadata, EncryptionPart};
use crate::fs_copy::CopyMetadata;
use crate::partition_reader::PartitionReader;
use aead::rand_core::RngCore;
use aead::OsRng;
use anyhow::{anyhow, Result};
use flate2::read::GzEncoder;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Check if a path is an existing directory
pub fn ensure_path_exists_and_is_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        // create path if it doesn't exist
        std::fs::create_dir_all(path)?;
    }
    if !path.is_dir() {
        return Err(anyhow!("Path is not a directory: {}", path.display()));
    }
    Ok(())
}

/// Check if a path is an existing empty directory
pub fn ensure_path_exists_and_is_empty_dir(path: &Path) -> Result<()> {
    ensure_path_exists_and_is_dir(path)?;
    if path.read_dir().unwrap().count() > 0 {
        return Err(anyhow!("Path is not empty: {}", path.display()));
    }
    Ok(())
}

#[derive(Debug)]
/// Enum that describes whether this file is a duplicate or not
pub enum DuplicateOrOriginal {
    /// This file is a duplicate of another file, here is the hash and location of the other file
    Duplicate(String, PathBuf),
    /// this file is an original file, here's its hash
    Original(String),
}

/// Implements test for DuplicateOrOriginal
impl DuplicateOrOriginal {
    /// Returns true if this is a duplicate
    pub(crate) fn is_original(&self) -> bool {
        match self {
            DuplicateOrOriginal::Duplicate(..) => false,
            DuplicateOrOriginal::Original(..) => true,
        }
    }
}

#[derive(Debug)]
/// information for how to chunk a file up
/// map the part number to which bytes need to be copied and where that part is going to end up.
pub struct PartitionGuidelines(pub(crate) HashMap<u64, ((u64, u64), PathBuf)>);

pub fn make_partition(
    file_len: u64,
    target_file_size: u64,
    file_path: PathBuf,
) -> Option<PartitionGuidelines> {
    if file_len <= target_file_size {
        return None;
    }
    let n_parts = (file_len as f64 / target_file_size as f64).ceil() as u64;
    let map = (0..n_parts)
        .map(
            // compute the boundaries of each segment
            |i| {
                let start = i * target_file_size;
                let end = if i == n_parts - 1 {
                    file_len
                } else {
                    (i + 1) * target_file_size
                };
                (i, ((start, end), file_path.join(format!("part_{i}"))))
            },
        )
        .collect::<HashMap<u64, ((u64, u64), PathBuf)>>();
    Some(PartitionGuidelines(map))
}

// Note (amiller68): The following is not used.
/*
   // TODO (laudiacay): Use proper function names
   // this comment lies in memoriam of the time i set these both to 10. if you estimate the disk
   // space used by a directory as only 512 bits, this would have filled 5 terabytes of disk space.
   // i'm not sure what i was thinking.
   pub fn make_big_filesystem_clusterfuck(depth_to_go: usize, width: usize, cwd: PathBuf) {
       if depth_to_go == 0 {
           for i in 0..width {
               let mut path = cwd.clone();
               path.push(format!("file{i}"));
               std::fs::File::create(path).unwrap();
           }
       } else {
           for i in 0..width {
               let mut path = cwd.clone();
               path.push(format!("dir{i}"));
               std::fs::create_dir(path.clone()).unwrap();
               make_big_filesystem_clusterfuck(depth_to_go - 1, width, path);
           }
       }
   }
*/

// TODO (xBalbinus & thea-exe) Our inline tests. Coordinate with laudiacay on what we need here
#[cfg(test)]
mod test {
    // Note (amiller68): Commented out seemingly irrelevant test
    // #[tokio::test]
    // async fn test_copy_paths_recursively() {
    //     use super::*;
    //     let tmp = tempfile::tempdir().unwrap();
    //     let scratch_root = tmp.path().join("scratch");
    //     let og_root = tmp.path().join("og");
    //     std::fs::create_dir(&scratch_root).unwrap();
    //     std::fs::create_dir(&og_root).unwrap();
    //     make_big_filesystem_clusterfuck(3, 3, og_root.clone());
    //     let paths = std::fs::read_dir(og_root.clone())
    //         .unwrap()
    //         .map(|res| res.unwrap().path())
    //         .collect::<Vec<PathBuf>>();
    //
    //     let stream = copy_paths_recursively(paths, scratch_root.clone(), false).await;
    //     let out_files = stream
    //         .map(|res| res.1.unwrap())
    //         .collect::<Vec<PathBuf>>()
    //         .await;
    //
    //     let in_files = FilesystemIterator::new(og_root, false)
    //         .await
    //         .collect::<Vec<PathBuf>>()
    //         .await;
    //
    //     assert_eq!(in_files.len() - 1, out_files.len());
    //     for file in out_files {
    //         let stripped =
    //             Path::new("/").join(file.strip_prefix(&scratch_root).unwrap().to_path_buf());
    //         assert!(in_files.contains(&stripped));
    //     }
    // }
}
