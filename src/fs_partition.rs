use crate::fs_copy::CopyMetadata;
use anyhow::Result;
use std::path::PathBuf;

use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::StreamExt;

#[derive(Clone, Debug)]
/// Enum that describes how and whether a file has been partitioned
pub enum MaybePartitioned {
    Partitioned(Vec<(u32, PathBuf)>),
    Unpartitioned(PathBuf),
}

#[derive(Debug)]
/// Metadata generated when a file is partitioned
pub struct PartitionMetadata {
    // TODO (laudiacay): This is never read. How do we want to use this?
    pub(crate) copy_metadata: CopyMetadata,
    /// Data on the file's partitions
    pub(crate) parts: MaybePartitioned,
}

// Note (amiller68): Not necessarily worried about CAR files for now, just make sure to use `targer-chunk-size` for determining the size of chunks
// How large to chunk files: realistically this should be slightly under 32 gigs (however much can fit into a car)
const MAX_FILE_SIZE: usize = 4 * 1024 * 1024 * 1024; // 4GB
const BUF_SIZE: usize = 1024 * 1024; // 1MB

// TODO (laudiacay): Make use of `target-chunk-size` for determining the size of chunks
/// Partition a file into part `part` of size `target_chunk_size`.
/// This is meant to be run in a tokio task, so be sure to use tokio::fs functions!
/// # Arguments
/// large_file: The path of the file to partition
/// part: What chunk of the file to partition
/// target_chunk_size: The size of the chunks to partition the file into
/// # Returns
/// The part index and path of the resulting partition
async fn do_chop(large_file: &PathBuf, part: u32) -> Result<(u32, PathBuf)> {
    let mut file = tokio::fs::File::open(&large_file).await?;
    // TODO (laudiacay): Handle the case where this path already exists
    let part_file_path = large_file.with_extension(format!("part-{part}"));
    let mut part_file = tokio::fs::File::create(part_file_path.clone()).await?;

    let mut buf = vec![0; BUF_SIZE];

    let mut bytes_read = 0;
    file.seek(tokio::io::SeekFrom::Start(
        part as u64 * MAX_FILE_SIZE as u64,
    ))
    .await?;
    while bytes_read < MAX_FILE_SIZE {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        part_file.write_all(&buf[..n]).await?;
        bytes_read += n;
    }
    Ok((part, part_file_path))
}

// TODO (laudiacay): Make use of 'target-chunk-size' to determine how many parts to chop into and how big
/// Partition a file into chunks of size `target_chunk_size`. If the file is smaller than `target_chunk_size`, it will not be partitioned.
/// This is meant to be run in a tokio task, so be sure to use tokio::fs functions!
/// # Arguments
/// copy_metadata: Metadata about the copied file to partition
/// # Returns
/// The metadata of the partitioned file
pub(crate) async fn partition_file(copy_metadata: CopyMetadata) -> Result<PartitionMetadata> {
    // If this is a directory, we don't need to partition it
    // TODO (laudiacay): Handle symlinks. They are not handled earlier in the copy process.
    if copy_metadata.original_metadata.is_dir() {
        let new_location = copy_metadata.new_location.clone();
        return Ok(PartitionMetadata {
            copy_metadata,
            parts: MaybePartitioned::Unpartitioned(new_location),
        });
    } else if copy_metadata.original_metadata.is_symlink() {
        todo!("Handle symlinks");
    }

    // Read the file size
    let file_size = copy_metadata.original_metadata.len();
    // If the file is smaller than the max file size,
    let parts = if file_size <= MAX_FILE_SIZE.try_into()? {
        // We don't need to partition it. Label as unpartitioned.
        MaybePartitioned::Unpartitioned(copy_metadata.new_location.clone())
    }
    // Otherwise we need to break this up
    else {
        // Determine how many parts we need to chop this file into
        let num_subfiles = (file_size as f64 / MAX_FILE_SIZE as f64).ceil() as u32;
        // Open streams to handle creating the subfiles
        let subfiles = tokio_stream::iter(0..num_subfiles);
        // Iterate over each stream, creating the subfiles
        let files_and_parts = subfiles.then(|i| do_chop(&copy_metadata.new_location, i));
        // Collect the results into a vector
        let ret: Vec<(u32, PathBuf)> = files_and_parts
            .collect::<Result<Vec<(u32, PathBuf)>>>()
            .await?;
        // Remove the original file
        tokio::fs::remove_file(&copy_metadata.new_location).await?;
        // Return the partition metadata
        MaybePartitioned::Partitioned(ret)
    };
    // Return the partition metadata
    Ok(PartitionMetadata {
        copy_metadata,
        parts,
    })
}

// TODO (xBalbinus & thea-exe): Our inline tests
#[cfg(test)]
mod test {
    #[test]
    fn test_do_chop() {
        todo!("test_do_chop")
    }

    #[test]
    fn test_partition_small_file() {
        todo!("test_partition_small_file")
    }

    #[test]
    fn test_partition_large_file() {
        todo!("test_partition_large_file")
    }

    /* etc. */
}
