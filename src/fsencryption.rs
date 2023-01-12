use anyhow::{anyhow, Result};
use async_stream::stream;
use std::fs;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_stream::{Stream, StreamExt};

struct EncryptionMetadata {
    original_file: PathBuf,
    part_number: Option<u32>,
    encrypted_file: PathBuf,
    encryption_key: String,
    cipher_info: String,
}

// TODO realistically this should be slightly under 32 gigs (however much can fit into a car)
const MAX_FILE_SIZE: usize = 4 * 1024 * 1024 * 1024; // 4GB

// TODO what if the file has another buddy next to it named .part2 or something?
pub(crate) async fn partition_file(
    large_file: PathBuf,
) -> impl Stream<Item = Result<(PathBuf, Option<u32>)>> {
    stream! {
    if fs::metadata(&large_file).unwrap().len() <= MAX_FILE_SIZE.try_into().unwrap() {
        yield Ok((large_file, None));
    } else {
        let mut part_number = 0;
        // open reader on file
        let mut file = tokio::fs::File::open(&large_file).await.unwrap();
        loop {
            // create new file
            let new_file = large_file.with_extension(format!("part{}", part_number));
            let mut new_file_handle = tokio::fs::File::create(&new_file).await.unwrap();
            // copy bytes from old file to new file
            let mut bytes_copied = 0;
            loop {
                let mut buf = [0; 1024];
                let bytes_read = file.read(&mut buf).await.unwrap();
                if bytes_read == 0 {
                    break;
                }
                new_file_handle.write_all(&buf[..bytes_read]).await.unwrap();
                bytes_copied += bytes_read;
                if bytes_copied >= MAX_FILE_SIZE {
                    break;
                }
            }
            // yield new file
            yield Ok((new_file, Some(part_number)));
            part_number += 1;
                // delete large file
                fs::remove_file(&large_file).unwrap();
            if bytes_copied < MAX_FILE_SIZE {
                break;
            }
        }
    }
        }
}

// TODO add support for more cipher modes
pub(crate) async fn encrypt_file_in_place(scratch_root: PathBuf) -> Result<EncryptionMetadata> {
    Err(anyhow!("unimplemented"))
}
