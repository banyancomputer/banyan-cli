use crate::encryption_writer::EncryptionWriter;
use crate::fs_partition::MaybePartitioned::{Partitioned, Unpartitioned};
use crate::fs_partition::PartitionMetadata;
use aead::OsRng;
use anyhow::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use futures::FutureExt;
use rand::RngCore;
use std::io::Write;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::StreamExt;

// How large a buffer to use for operating on files
const BUF_SIZE: usize = 1024 * 1024; // 1MB
                                     // How large a file can safely be in order to encrypt it.
const MAX_SAFE_ENCRYPTION_SIZE: usize = 34_359_738_368; // 32 gigs, the GCM safe limit

// TODO (laudiacay): Do we need to keep track of nonces?
#[derive(Debug)]
/// Metadata generated when a part of a file is encrypted and compressed
pub struct EncryptionPart {
    /// Where part or file was originally located
    pub old_file_path: PathBuf,
    /// Where the encrypted and compressed part or file is located
    pub encrypted_file_path: PathBuf,
    /// The key used to encrypt the part or file
    pub key: [u8; 32],
    /// The size after compression
    pub size_after: u64,
}

#[derive(Debug)]
/// Metadata generated when a file is compressed and encrypted
pub struct EncryptionMetadata {
    /// The original partition of the file
    partition_metadata: PartitionMetadata,
    /// The parts of the file that were encrypted and associated metadata
    encrypted_keys: Vec<EncryptionPart>,
    /// The cipher used to encrypt the file
    cipher_info: String,
    /// The compression used to compress the file
    compression_info: String,
}

// TODO (laudiacay): Filenames need to randomized and unique
// TODO (laudiacay): Make use of `target-chunk-size` for determining whether or not to encrypt
// TODO (laudiacay): Make sure that keys aren't owned by this process. Make sure they're zeroed from memory.
/// Compress and encrypt a file
/// This will be runned in a tokio task, so be sure to use tokio::fs functions!
/// # Arguments
/// path: The path of the file to compress and encrypt
async fn compress_and_encrypt_file(path: PathBuf) -> Result<([u8; 32], PathBuf, usize)> {
    // Create a new random key
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    // Open the file using the path
    let mut file = tokio::fs::File::open(&path).await?;
    // Determine whether or not the file is small enough to be used with our EncryptionWriter
    let file_length = file.metadata().await?.len() as usize;
    assert!(file_length <= MAX_SAFE_ENCRYPTION_SIZE);
    // Open a new file to write the compressed and encrypted data to
    // TODO (laudiacay): fix async io up... and/or buffering...
    let compressed_encrypted_file_path = path.with_extension("gzip.enc");
    let mut compressed_encrypted_file =
        std::fs::File::create(compressed_encrypted_file_path.clone())?;
    // Declare a new GzEncoder new Encryption Writer with our new file and key
    let encrypted_writer = EncryptionWriter::new(&mut compressed_encrypted_file, &key);

    // Declare buffers
    let mut buf_for_input = vec![0_u8; BUF_SIZE];
    let nice_clean_zero_buf = vec![0_u8; BUF_SIZE];
    let mut bytes_read = 0_usize;

    // Compress the file using the GzEncoder on top of the EncryptionWriter
    // TODO (laudiacay): Make compressions better. This is fine for now to get it working.
    let mut gzencoder = GzEncoder::new(encrypted_writer, Compression::default());

    // Writes the file using our compression encoder
    while bytes_read < file_length {
        // Read in BUF_SIZE bytes from the file
        let n = file.read(&mut buf_for_input[0..(BUF_SIZE)]).await?;
        if n == 0 {
            break;
        }
        // Write the bytes to the encoder
        gzencoder.write_all(&buf_for_input[..n])?;
        // Zero out the buffer
        gzencoder.flush()?;
        // Look back and zero out the file, flush it
        file.seek(std::io::SeekFrom::Start(bytes_read as u64))
            .await?;
        // TODO (laudiacay): Make sure write actually writes the whole buffer. Idt it does.
        let should_be_n = file.write(&nice_clean_zero_buf[..n]).await?;
        assert_eq!(should_be_n, n);
        file.flush().await?;
        // Note (amiller68):  What's going on here?
        let current_location = file.seek(std::io::SeekFrom::Current(0)).await?;
        assert_eq!(current_location, (bytes_read as u64) + (n as u64));
        bytes_read += n;
    }

    // Finish the compression
    let encryptor = gzencoder.finish()?;
    // Finish the encryption
    let bytes_written = encryptor.finish()?;

    // TODO (laudiacay): When we figure out how operate on files in place, replace with "wipe the rest of the file"
    // TODO (laudiacay): Wipe pre-partition file properly
    // Remove the original file
    tokio::fs::remove_file(path).await?;

    // Return the key and the path to the new file and the number of bytes written
    Ok((key, compressed_encrypted_file_path, bytes_written))
}

// TODO (laudiacay): add support for more cipher modes
/// Compress and a partitioned file
/// This will be runned in a tokio task, so be sure to use tokio::fs functions!
/// # Arguments
/// partition_metadata: The metadata of the (maybe) partitioned file
pub(crate) async fn compress_and_encrypt_partitioned_file(
    partition_metadata: PartitionMetadata,
) -> Result<EncryptionMetadata> {
    // TODO (laudiacay): implement map for partitioned and unpartitioned files
    // Depending on the partition type, we'll need to do different things
    let encrypted_parts_and_keys = match partition_metadata.parts.clone() {
        // If the file is partitioned, we'll need to encrypt each partition
        Partitioned(parts) => {
            // Create an iterating stream of the parts
            tokio_stream::iter(parts)
                // Map the parts to a future that will encrypt and compress the part
                .then(|(_part_num, path)| {
                    // Note (amiller68):  Does `move` stop the underlying return value from being returned?
                    // TODO have this return encryptionpart
                    compress_and_encrypt_file(path.clone()).map(move |res| {
                        let (key, encrypted_part, length) = res.unwrap();
                        EncryptionPart {
                            old_file_path: path,
                            encrypted_file_path: encrypted_part,
                            key,
                            size_after: length as u64,
                        }
                    })
                })
                // Collect the results into a vector
                .collect::<Vec<_>>()
                .await
        }
        // If the file is not partitioned, we'll just encrypt and compress the file
        Unpartitioned(path) => {
            let (key, encrypted_file_path, bytes_written) =
                compress_and_encrypt_file(path.clone()).await?;
            // Create a vector with the single result
            vec![EncryptionPart {
                old_file_path: path.clone(),
                encrypted_file_path,
                key,
                size_after: bytes_written as u64,
            }]
        }
    };
    // Return the metadata on the encrypted file and its (maybe) partitions
    Ok(EncryptionMetadata {
        partition_metadata,
        encrypted_keys: encrypted_parts_and_keys,
        // TODO (laudiacay): add support for more cipher and encryption modes
        cipher_info: "AES256GCM".to_string(),
        compression_info: "GZIP".to_string(),
    })
}

// TODO (xBalbinus & thea-exe): Our inline tests
// Note (amiller68): Testing may rely on decrypting the file, which is not yet implemented
#[cfg(test)]
mod test {
    #[test]
    fn test() {
        todo!("Test compression and encryption");
    }
}
