use anyhow::{anyhow, Result};
use futures::FutureExt;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio_stream::StreamExt;

use crate::fs_partition::PartitionMetadata;

use crate::fs_partition::MaybePartitioned::{Partitioned, Unpartitioned};
// use aead::consts::U32;
use aead::stream::{Encryptor, StreamBE32, StreamPrimitive};
use aead::{rand_core::RngCore, stream::NewStream, OsRng};
//use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{Aes256Gcm, KeyInit};

pub struct EncryptionMetadata {
    original_file: PathBuf,
    encrypted_parts_and_keys: Vec<(u32, PathBuf, [u8; 32])>,
    cipher_info: String,
}

// TODO ownership of keys is probably wrong and bad? check on that and make sure they're zeroed from memory
async fn encrypt_one_part(to_encrypt_in_place: PathBuf) -> Result<([u8; 32], PathBuf)> {
    // key
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    // nonce
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    // aad
    let associated_data = b"";
    // cipher
    let cipher = Aes256Gcm::new(key.as_ref().into());
    let mut stream_encryptor = StreamBE32::from_aead(cipher, nonce.as_ref().into()).encryptor();

    // open file
    let mut file = tokio::fs::File::open(&to_encrypt_in_place).await?;
    let file_length = file.metadata().await?.len() as usize;
    assert!(file_length <= 4 * 1024 * 1024 * 1024); // GCM safe limit, 2**32

    // open output
    let encrypted_file_path = to_encrypt_in_place.with_extension("enc");
    let mut encrypted_file = tokio::fs::File::create(encrypted_file_path.clone()).await?;

    let mut buf = vec![0; 1024 * 1024];
    let nice_clean_zero_buf = vec![0; 1024 * 1024];
    let mut bytes_read = 0_usize;

    while bytes_read < file_length {
        // read in 1MB- there is a bunch of space in the end for the auth tag
        let n = file.read(&mut buf[0..(1024 * 1024)]).await?;

        if n == 0 {
            break;
        }

        // are we at the end of the file?
        if bytes_read + n < file_length {
            Encryptor::encrypt_next_in_place(&mut stream_encryptor, associated_data, &mut buf)
                .map_err(|_| anyhow!("encryption error"))?;
            encrypted_file.write_all(&buf[..n]).await?;
        } else {
            Encryptor::encrypt_next_in_place(&mut stream_encryptor, associated_data, &mut buf)
                .map_err(|_| anyhow!("encryption error"))?;
        };

        // TODO this sucks and is *probably* wrong with the AEAD tag. wait until this PR lands:
        // TODO https://github.com/RustCrypto/traits/pull/1189

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

    file.flush().await?;

    tokio::fs::remove_file(to_encrypt_in_place).await?;

    Ok((key, encrypted_file_path))
}

// TODO add support for more cipher modes
pub(crate) async fn encrypt_file_in_place(
    partition_metadata: PartitionMetadata,
) -> Result<EncryptionMetadata> {
    let encrypted_parts_and_keys = match partition_metadata.parts {
        Partitioned(parts) => {
            tokio_stream::iter(parts)
                .then(|(part_num, path)| {
                    encrypt_one_part(path).map(move |res| {
                        let (key, encrypted_part) = res.unwrap();
                        (part_num, encrypted_part, key)
                    })
                })
                .collect::<Vec<_>>()
                .await
        }
        Unpartitioned(path) => {
            let (key, encrypted_file_path) = encrypt_one_part(path).await?;
            vec![(0, encrypted_file_path, key)]
        }
    };
    Ok(EncryptionMetadata {
        original_file: partition_metadata.original_file,
        encrypted_parts_and_keys,
        cipher_info: "AES256GCM".to_string(),
    })
}
