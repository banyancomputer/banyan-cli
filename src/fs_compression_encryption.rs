use anyhow::Result;
use std::io::Write;
use std::path::PathBuf;

use futures::FutureExt;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::StreamExt;

use flate2::write::GzEncoder;
use flate2::Compression;

use crate::fs_partition::MaybePartitioned::{Partitioned, Unpartitioned};
use crate::fs_partition::PartitionMetadata;

use crate::encryption_writer::EncryptionWriter;
use aead::OsRng;
use rand::RngCore;

pub struct EncryptionPart {
    pub old_file_path: PathBuf,
    pub encrypted_file_path: PathBuf,
    pub key: [u8; 32],
    pub size_after: u64,
}

pub struct EncryptionMetadata {
    partition_metadata: PartitionMetadata,
    /// ordered the same as the parts array
    encrypted_keys: Vec<EncryptionPart>,
    cipher_info: String,
    compression_info: String,
}

// TODO @xh @thea idk how to test this hmm maybe wait until decryption working
// TODO @xh @thea you should go through the entire repo and attempt to get rid of unwraps
// TODO ownership of keys is probably wrong and bad? check on that and make sure they're zeroed from memory.
// TODO what do we think about literally encrypting the file in place with one file handle. why don't we do that. ya goofin
async fn compress_and_encrypt_one_part(
    to_encrypt_in_place: PathBuf,
) -> Result<([u8; 32], PathBuf, usize)> {
    // key
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    // nonce
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);

    // open file
    let mut file = tokio::fs::File::open(&to_encrypt_in_place).await?;
    let file_length = file.metadata().await?.len() as usize;
    assert!(file_length <= 4 * 1024 * 1024 * 1024); // GCM safe limit, 2**32

    // open output
    // TODO fix async io up... and/or buffering...
    let compressed_encrypted_file_path = to_encrypt_in_place.with_extension("gzip.enc");
    let mut compressed_encrypted_file =
        std::fs::File::create(compressed_encrypted_file_path.clone())?;
    let encrypted_writer = EncryptionWriter::new(&mut compressed_encrypted_file, &key);

    let mut buf_for_input = vec![0_u8; 1024 * 1024];
    let nice_clean_zero_buf = vec![0_u8; 1024 * 1024];
    let mut bytes_read = 0_usize;

    // compress
    // TODO compression per part is not great compression- we could get a lot better. but it's a start.
    let mut gzencoder = GzEncoder::new(encrypted_writer, Compression::default());

    while bytes_read < file_length {
        // read in 1MB- there is a bunch of space in the end for the auth tag
        let n = file.read(&mut buf_for_input[0..(1024 * 1024)]).await?;

        if n == 0 {
            break;
        }

        gzencoder.write_all(&buf_for_input[..n])?;
        gzencoder.flush()?;

        // look back and zero out the file, flush it
        file.seek(std::io::SeekFrom::Start(bytes_read as u64))
            .await?;
        let should_be_n = file.write(&nice_clean_zero_buf[..n]).await?;
        assert_eq!(should_be_n, n);
        file.flush().await?;
        let current_location = file.seek(std::io::SeekFrom::Current(0)).await?;
        assert_eq!(current_location, (bytes_read as u64) + (n as u64));

        bytes_read += n;
    }

    let encryptor = gzencoder.finish()?;
    let bytes_written = encryptor.finish()?;

    // TODO remove this once you do things in place- replace with "wipe the rest of the file"
    tokio::fs::remove_file(to_encrypt_in_place).await?;

    Ok((key, compressed_encrypted_file_path, bytes_written))
}

// TODO add support for more cipher modes
pub(crate) async fn compress_and_encrypt_file_in_place(
    partition_metadata: PartitionMetadata,
) -> Result<EncryptionMetadata> {
    let encrypted_parts_and_keys = match partition_metadata.parts.clone() {
        Partitioned(parts) => {
            tokio_stream::iter(parts)
                .then(|(_part_num, path)| {
                    compress_and_encrypt_one_part(path.clone()).map(move |res| {
                        let (key, encrypted_part, length) = res.unwrap();
                        EncryptionPart {
                            old_file_path: path,
                            encrypted_file_path: encrypted_part,
                            key,
                            size_after: length as u64,
                        }
                    })
                })
                .collect::<Vec<_>>()
                .await
        }
        // TODO implement map for partitioned and unpartitioned
        Unpartitioned(path) => {
            let (key, encrypted_file_path, bytes_written) =
                compress_and_encrypt_one_part(path.clone()).await?;
            vec![EncryptionPart {
                old_file_path: path.clone(),
                encrypted_file_path,
                key,
                size_after: bytes_written as u64,
            }]
        }
    };
    Ok(EncryptionMetadata {
        partition_metadata,
        encrypted_keys: encrypted_parts_and_keys,
        cipher_info: "AES256GCM".to_string(),
        compression_info: "GZIP".to_string(),
    })
}
