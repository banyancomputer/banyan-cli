use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_stream::{StreamExt};

pub struct EncryptionMetadata {
    original_file: PathBuf,
    parts: Vec<PathBuf>,
    encrypted_parts: Vec<PathBuf>,
    encryption_key: String,
    cipher_info: String,
}

// TODO realistically this should be slightly under 32 gigs (however much can fit into a car)
const MAX_FILE_SIZE: usize = 4 * 1024 * 1024 * 1024; // 4GB
const BUF_SIZE: usize = 1024 * 1024; // 1MB

async fn do_chop(large_file: PathBuf, part: u32) -> Result<(PathBuf, Option<u32>)> {
    let mut file = tokio::fs::File::open(&large_file).await?;
    let mut buf = vec![0; BUF_SIZE];
    let part_file_path = large_file.with_extension(format!(
        "part-{}",
        part
    ));
    let mut part_file = tokio::fs::File::create(part_file_path.clone()).await?;
    let mut bytes_read = 0;
    while bytes_read < MAX_FILE_SIZE {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        part_file.write_all(&buf[..n]).await?;
        bytes_read += n;
    }
    Ok((part_file_path, Some(part)))
}

// TODO what if the file has another buddy next to it named .part2 or something?
pub(crate) async fn partition_file(large_file: PathBuf) -> Result<Vec<(PathBuf, Option<u32>)>> {
    let file_size = fs::metadata(&large_file)?.len();
    if file_size <= MAX_FILE_SIZE.try_into()? {
        Ok(vec![(large_file, None)])
    } else {
        // open reader on file
        let num_subfiles =
            (file_size as f64 / MAX_FILE_SIZE as f64).ceil() as u32;
        let subfiles = tokio_stream::iter(0..num_subfiles);
        let files_and_parts = subfiles.then(|i| do_chop(large_file.clone(), i));
        let ret : Result<Vec<(PathBuf, Option<u32>)>> = files_and_parts.collect::<Result<Vec<(PathBuf, Option<u32>)>>>().await;
        tokio::fs::remove_file(large_file).await?;
        ret
    }
}

// TODO add support for more cipher modes
pub(crate) async fn encrypt_file_in_place(file_data: Vec<(PathBuf, Option<u32>)>) -> Result<EncryptionMetadata> {
    Err(anyhow!("unimplemented"))
}
