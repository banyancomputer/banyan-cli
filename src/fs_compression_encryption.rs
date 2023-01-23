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

async fn do_copy(copy_metadata: Rc<CopyMetadata>, part: u64) -> Result<EncryptionPart> {
    // to get to this point it needs to be an original file and have some partition guidelines- just check one more time!
    assert!(copy_metadata.duplicate_or_original.is_original());
    assert!(copy_metadata.partition_guidelines.is_some());
    let (segment, new_path) = copy_metadata
        .as_ref()
        .partition_guidelines
        .as_ref()
        .map(|pg| pg.0.get(&part).unwrap())
        .unwrap();

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

pub async fn process_copy_metadata(copy_metadata: CopyMetadata) -> Result<EncryptionMetadata> {
    let mut encrypted_pieces = Vec::new();
    // cases where you shouldn't copy: it's a duplicate, it's a directory, or it's a symlink. These will be annotated in the metadata, but not backed up.
    if !copy_metadata.duplicate_or_original.is_original()
        || copy_metadata.original_metadata.is_dir()
        || copy_metadata.original_metadata.is_symlink()
    {
        return Ok(EncryptionMetadata {
            copy_metadata: Rc::new(copy_metadata),
            encrypted_pieces: None,
            cipher_info: "".to_string(),
            compression_info: "".to_string(),
        });
    }
    // TODO add better types to make it so that only non-duplicate files get here :)
    // assert that we have partition guidelines
    assert!(copy_metadata.partition_guidelines.is_some());
    // TODO (laudiacay): make this a parallel for loop or stream
    let copy_metadata = Rc::new(copy_metadata);
    for part in copy_metadata
        .partition_guidelines
        .as_ref()
        .unwrap()
        .0
        .keys()
    {
        encrypted_pieces.push(do_copy(copy_metadata.clone(), *part).await?);
    }
    Ok(EncryptionMetadata {
        copy_metadata,
        encrypted_pieces: Some(encrypted_pieces),
        cipher_info: "AES-256-GCM".to_string(),
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
