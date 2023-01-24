use anyhow::anyhow;
use jwalk::DirEntry;
use std::fmt::Debug;
use std::fs::Metadata;
use std::path::PathBuf;
use std::rc::Rc;

// TODO(laudiacay): split these up into submodules

#[derive(Debug)]
pub struct SpiderMetadata {
    pub(crate) original_root: PathBuf,
    pub(crate) original_location: DirEntry<((), ())>,
    pub(crate) canonicalized_path: PathBuf,
    pub(crate) original_metadata: Metadata,
}

impl From<DirEntry<((), ())>> for SpiderMetadata {
    fn from(entry: DirEntry<((), ())>) -> Self {
        let original_root = entry.path().parent().unwrap().to_path_buf();
        let original_location = entry;
        let canonicalized_path = original_location.path().canonicalize().unwrap();
        let original_metadata = original_location.metadata().unwrap();
        SpiderMetadata {
            original_root,
            original_location,
            canonicalized_path,
            original_metadata,
        }
    }
}

// TODO (laudiacay) make changeable and better
pub struct CompressionMetadata {
    /// string describing compression algorithm
    pub(crate) compression_info: String,
    /// size after compression
    pub(crate) size_after: u64,
}

#[derive(Debug, Clone)]
pub struct PartitionMetadata {
    /// The size of the chunks
    pub(crate) chunk_size: u64,
}

#[derive(Debug)]
/// Metadata generated when a part of a file is encrypted and compressed
pub struct EncryptionPart {
    /// The key used to encrypt the part or file
    pub key: [u8; 32],
    /// The nonce used to encrypt the part or file
    pub nonce: [u8; 12],
    /// The size after encryption
    pub size_after: u64,
}

#[derive(Debug)]
/// Metadata generated when a file is compressed and encrypted
pub struct EncryptionMetadata {
    /// The parts of the file that were encrypted and associated metadata
    pub(crate) encrypted_pieces: Vec<EncryptionPart>,
    /// The cipher used to encrypt the file
    pub(crate) cipher_info: String,
}

#[derive(Debug)]
/// Metadata that is emitted on successful write into new filesystem
pub struct WriteoutMetadata {
    /// mapping from compressed and encrypted chunks to their new locations
    pub(crate) chunk_locations: Vec<PathBuf>,
}

// /// this struct is used to build up the data processing steps for a file
// pub struct DataProcessBuilder {
//     /// describes how we compressed the file
//     compression: Option<CompressionMetadata>,
//     /// describes how we partitioned the file
//     partition: Option<PartitionMetadata>,
//     /// describes how we encrypted the file
//     encryption: Option<EncryptionMetadata>,
//     /// describes how we wrote the file out on the new filesystem
//     writeout: Option<WriteoutMetadata>,
// }

/// this struct is the completed data processing steps for a file
pub struct DataProcess {
    /// describes how we compressed the entire file
    pub(crate) compression: CompressionMetadata,
    /// describes how we partitioned the file into chunks (each slice is encrypted and written out
    /// separately- should be ENCRYPTION_TAG_SIZE bytes less than target_chunk_size!)
    pub(crate) partition: PartitionMetadata,
    /// describes how we encrypted the file
    pub(crate) encryption: EncryptionMetadata,
    /// describes how/where we wrote the file out on the new filesystem
    pub(crate) writeout: WriteoutMetadata,
}

/// this struct is used to build up the data processing steps for a file
#[derive(Debug, Clone)]
pub struct DataProcessPlan {
    /// describes how we will compress the file
    pub(crate) compression: CompressionPlan,
    /// describes how we will partition the file
    pub(crate) partition: PartitionPlan,
    /// describes how we will encrypt the file
    pub(crate) encryption: EncryptionPlan,
    /// describes how we will write the file out on the new filesystem
    pub(crate) writeout: WriteoutPlan,
}

#[derive(Debug, Clone)]
pub struct CompressionPlan {
    compression_info: String,
}
impl CompressionPlan {
    pub(crate) fn new_gzip() -> Self {
        CompressionPlan {
            compression_info: "gzip".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PartitionPlan(pub(crate) PartitionMetadata);

impl PartitionPlan {
    pub fn new_from_chunk_size(chunk_size: u64) -> Self {
        PartitionPlan(PartitionMetadata { chunk_size })
    }
}

#[derive(Debug, Clone)]
pub struct EncryptionPlan {
    pub(crate) cipher_info: String,
    // TODO yikes
    pub(crate) tag_size: u64,
}

impl EncryptionPlan {
    pub fn new_aes_256_gcm() -> Self {
        EncryptionPlan {
            cipher_info: "aes-256-gcm".to_string(),
            tag_size: 16,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WriteoutPlan {
    pub(crate) output_dir: PathBuf,
}

/// This struct is used to describe how a file was processed. Either it was a duplicate/symlink/
/// directory and there isn't much to do, or else we need to go through compression, partition, and
/// encryption steps.
pub enum DataProcessDirective<T> {
    /// The file was a duplicate, use the processed data from the original- here's where to find it
    /// once everything else is restored
    Duplicate(Rc<SpiderMetadata>),
    /// It was a directory, just create it
    Directory,
    /// it was a symlink, just create it
    Symlink,
    /// it was a file, here's the metadata for how it was encrypted and compressed
    File(T),
}

// all these are no-ops except for the File case
impl TryInto<DataProcessDirective<DataProcess>> for DataProcessDirective<DataProcessPlan> {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<DataProcessDirective<DataProcess>, Self::Error> {
        match self {
            DataProcessDirective::Duplicate(d) => Ok(DataProcessDirective::Duplicate(d)),
            DataProcessDirective::Directory => Ok(DataProcessDirective::Directory),
            DataProcessDirective::Symlink => Ok(DataProcessDirective::Symlink),
            DataProcessDirective::File(_) => {
                Err(anyhow!("DataProcessDirective::File not implemented yet"))
            }
        }
    }
}

pub struct PipelinePlan {
    /// describes where a file came from on the original filesystem
    pub(crate) origin_data: Rc<SpiderMetadata>,
    /// describes data processing, if any is needed
    pub(crate) data_processing: DataProcessDirective<DataProcessPlan>,
}

/// describes how a file from the origin was processed.
pub struct Pipeline {
    /// describes where a file came from on the original filesystem
    pub(crate) origin_data: Rc<SpiderMetadata>,
    /// describes data processing, if any is needed
    pub(crate) data_processing: DataProcessDirective<DataProcess>,
}

impl TryInto<Pipeline> for PipelinePlan {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Pipeline, Self::Error> {
        let PipelinePlan {
            origin_data,
            data_processing,
        } = self;
        let data_processing = data_processing.try_into()?;
        Ok(Pipeline {
            origin_data,
            data_processing,
        })
    }
}
