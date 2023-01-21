use crate::encryption_writer::EncryptionWriter;
use crate::fs_copy::CopyMetadata;
use crate::partition_reader::PartitionReader;
use aead::OsRng;
use anyhow::Result;
use flate2::write::GzEncoder;
use rand::RngCore;
use std::path::PathBuf;
use std::rc::Rc;
use tokio::fs::File;

// How large a buffer to use for operating on files
const BUF_SIZE: usize = 1024 * 1024; // 1MB
                                     // How large a file can safely be in order to encrypt it.

// TODO (laudiacay): Do we need to keep track of nonces?
#[derive(Debug)]
/// Metadata generated when a part of a file is encrypted and compressed
pub struct EncryptionPart {
    /// Segment identifier for the part of the file
    pub segment: (u64, u64),
    /// Where the encrypted and compressed part or file is located
    pub encrypted_file_path: PathBuf,
    /// The key used to encrypt the part or file
    pub key: [u8; 32],
    /// The size after compression and encryption
    pub size_after: u64,
    /// The cipher used to encrypt the file
    pub cipher_info: String,
    /// The compression used to compress the file
    pub compression_info: String,
}

#[derive(Debug)]
/// Metadata generated when a file is compressed and encrypted
pub struct EncryptionMetadata {
    /// The data so far from the file informing how it will be copied over
    copy_metadata: Rc<CopyMetadata>,
    /// The parts of the file that were encrypted and associated metadata
    encrypted_pieces: Option<Vec<EncryptionPart>>,
    /// The cipher used to encrypt the file
    cipher_info: String,
    /// The compression used to compress the file
    compression_info: String,
}

async fn do_copy(copy_metadata: Rc<CopyMetadata>, part: u32) -> Result<EncryptionPart> {
    // to get to this point it needs to be an original file and have some partition guidelines- just check one more time!
    assert!(copy_metadata.duplicate_or_original.is_original());
    assert!(copy_metadata.partition_guidelines.is_some());
    let (segment, new_path) = copy_metadata.as_ref()
        .partition_guidelines.as_ref()
        .map(|pg| {
            pg.0.get(&part.into()).unwrap()
        }).unwrap();


    let mut old_file_reader = PartitionReader::new_from_path(
        segment,
        copy_metadata
            .original_root
            .join(copy_metadata.original_location.file_name.clone()),
    )
    .await?;
    let mut new_file_writer = File::open(new_path.clone()).await?;
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    let new_file_encryptor = EncryptionWriter::new(&mut new_file_writer, &key);
    let cipher_info = new_file_encryptor.cipher_info().clone();
    let mut new_file_compressor =
        GzEncoder::new(new_file_encryptor, flate2::Compression::default());
    // TODO (laudiacay): look at whether you want to use asyncread and asyncwrite all throughout...? definitely should to throughputmaxx, but not for now.
    std::io::copy(&mut old_file_reader, &mut new_file_compressor)?;
    // finish the gzip compression and write it to the encryptor, finish the encryptor and write it to the file, done
    let bytes_written = new_file_compressor.finish()?.finish().await?;
    Ok(EncryptionPart {
        segment: *segment,
        encrypted_file_path: (*new_path.clone()).to_owned(),
        key,
        size_after: bytes_written as u64,
        cipher_info,
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
