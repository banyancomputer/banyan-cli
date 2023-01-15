use crate::fs_copy::{CopyMetadata, DuplicateOrOriginal};
use anyhow::Result;
use jwalk::DirEntry;
use std::fs::Metadata;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_stream::StreamExt;

#[derive(Clone)]
pub enum MaybePartitioned {
    Partitioned(Vec<(u32, PathBuf)>),
    Unpartitioned(PathBuf),
}

pub struct PartitionMetadata {
    pub(crate) original_root: PathBuf,
    pub(crate) original_location: DirEntry<(u64, Option<u64>)>,
    pub(crate) original_metadata: Metadata,
    duplicate_or_original: DuplicateOrOriginal,
    location_in_new_fs_before_partition: PathBuf,
    pub(crate) parts: MaybePartitioned,
    blake2_file_hash: Option<String>,
    file_size: Option<u64>,
}

// TODO realistically this should be slightly under 32 gigs (however much can fit into a car)
const MAX_FILE_SIZE: usize = 4 * 1024 * 1024 * 1024; // 4GB
const BUF_SIZE: usize = 1024 * 1024; // 1MB

// TODO TEST TEST TEST TEST @xiangan @thea
// TODO what if the file has another buddy next to it named .part2 or something?
async fn do_chop(large_file: &PathBuf, part: u32) -> Result<(u32, PathBuf)> {
    let mut file = tokio::fs::File::open(&large_file).await?;
    let part_file_path = large_file.with_extension(format!("part-{part}"));
    let mut part_file = tokio::fs::File::create(part_file_path.clone()).await?;

    let mut buf = vec![0; BUF_SIZE];

    let mut bytes_read = 0;
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

// TODO TEST TEST TEST TEST @xiangan @thea
pub(crate) async fn partition_file(copy_metadata: CopyMetadata) -> Result<PartitionMetadata> {
    if copy_metadata.original_metadata.is_dir() || copy_metadata.original_metadata.is_symlink() {
        return Ok(PartitionMetadata {
            original_root: copy_metadata.original_root,
            original_location: copy_metadata.original_location,
            original_metadata: copy_metadata.original_metadata,
            duplicate_or_original: copy_metadata.duplicate_or_original,
            location_in_new_fs_before_partition: copy_metadata.new_location.clone(),
            parts: MaybePartitioned::Unpartitioned(copy_metadata.new_location),
            blake2_file_hash: copy_metadata.blake2_file_hash,
            file_size: None,
        });
    };
    let file_size = copy_metadata.original_metadata.len();
    let parts = if file_size <= MAX_FILE_SIZE.try_into()? {
        MaybePartitioned::Unpartitioned(copy_metadata.new_location.clone())
    } else {
        // open reader on file
        let num_subfiles = (file_size as f64 / MAX_FILE_SIZE as f64).ceil() as u32;
        let subfiles = tokio_stream::iter(0..num_subfiles);
        let files_and_parts = subfiles.then(|i| do_chop(&copy_metadata.new_location, i));
        let ret: Vec<(u32, PathBuf)> = files_and_parts
            .collect::<Result<Vec<(u32, PathBuf)>>>()
            .await?;
        tokio::fs::remove_file(&copy_metadata.new_location).await?;
        MaybePartitioned::Partitioned(ret)
    };
    Ok(PartitionMetadata {
        original_root: copy_metadata.original_root,
        original_location: copy_metadata.original_location,
        location_in_new_fs_before_partition: copy_metadata.new_location,
        parts,
        blake2_file_hash: copy_metadata.blake2_file_hash,
        file_size: Some(file_size),
        duplicate_or_original: DuplicateOrOriginal::Original,
        original_metadata: copy_metadata.original_metadata,
    })
}
