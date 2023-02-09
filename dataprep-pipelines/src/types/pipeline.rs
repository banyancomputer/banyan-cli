use crate::types::{
    plan::{DataProcessPlan, PipelinePlan},
    shared::DataProcessDirective,
    spider::SpiderMetadata,
};
use age::secrecy::ExposeSecret;
use anyhow::anyhow;
use std::fmt::Debug;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;

use crate::types::shared::DataProcessDirectiveToDisk;
use crate::types::spider::SpiderMetadataToDisk;
use serde::{Deserialize, Serialize};

// TODO (laudiacay) this "ToDisk" stuff sort of sucks

// TODO (laudiacay) make changeable and better
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionMetadata {
    /// string describing compression algorithm
    pub compression_info: String,
    /// size after compression
    pub size_after: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionMetadata {
    /// The size of the chunks
    pub chunk_size: u64,
    /// number of chunks
    pub num_chunks: u64,
}

fn serialize_age_identity<S>(
    identity: &age::x25519::Identity,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(identity.to_string().expose_secret())
}

fn deserialize_age_identity<'de, D>(deserializer: D) -> Result<age::x25519::Identity, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    age::x25519::Identity::from_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Clone, Serialize, Deserialize)]
/// Metadata generated when a part of a file is encrypted and compressed
pub struct EncryptionPart {
    /// age identity for decrypting this part
    #[serde(
        serialize_with = "serialize_age_identity",
        deserialize_with = "deserialize_age_identity"
    )]
    pub identity: age::x25519::Identity,
}

#[derive(Clone, Serialize, Deserialize)]
/// Metadata generated when a file is compressed and encrypted
pub struct EncryptionMetadata {
    /// The parts of the file that were encrypted and associated metadata
    pub encrypted_pieces: Vec<EncryptionPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Metadata that is emitted on successful write into new filesystem
pub struct WriteoutMetadata {
    /// mapping from compressed and encrypted chunks to their new locations
    pub chunk_locations: Vec<PathBuf>,
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
#[derive(Clone, Serialize, Deserialize)]
pub struct DataProcess {
    /// describes how we compressed the entire file
    pub compression: CompressionMetadata,
    /// describes how we partitioned the file into chunks (each slice is encrypted and written out
    /// separately- should be ENCRYPTION_TAG_SIZE bytes less than target_chunk_size!)
    pub partition: PartitionMetadata,
    /// describes how we encrypted the file
    pub encryption: EncryptionMetadata,
    /// describes how/where we wrote the file out on the new filesystem
    pub writeout: WriteoutMetadata,
}

// all these are no-ops except for the File case
impl TryFrom<DataProcessDirective<DataProcessPlan>> for DataProcessDirective<DataProcess> {
    type Error = anyhow::Error;

    fn try_from(
        data_process_directive: DataProcessDirective<DataProcessPlan>,
    ) -> Result<Self, Self::Error> {
        match data_process_directive {
            DataProcessDirective::File(_) => Err(anyhow!("You have to process files!")),
            DataProcessDirective::Directory => Ok(DataProcessDirective::Directory),
            DataProcessDirective::Symlink => Ok(DataProcessDirective::Symlink),
            DataProcessDirective::Duplicate(spider) => Ok(DataProcessDirective::Duplicate(spider)),
        }
    }
}

/// describes how a file from the origin was processed.
#[derive(Clone)]
pub struct Pipeline {
    /// describes where a file came from on the original filesystem
    pub origin_data: Rc<SpiderMetadata>,
    /// describes data processing, if any is needed
    pub data_processing: DataProcessDirective<DataProcess>,
}

impl TryFrom<PipelinePlan> for Pipeline {
    type Error = anyhow::Error;

    fn try_from(pipeline_plan: PipelinePlan) -> Result<Self, Self::Error> {
        let origin_data = pipeline_plan.origin_data;
        let data_processing = pipeline_plan.data_processing.try_into()?;
        Ok(Pipeline {
            origin_data,
            data_processing,
        })
    }
}

pub struct PipelineToCar {
    pipeline: Pipeline,
    car_id: Cid,
}
// TODO deduplicate by removing things from todisk earlier. this is silly
#[derive(Clone, Serialize, Deserialize)]
pub struct PipelineToDisk {
    /// describes where a file came from on the original filesystem
    pub origin_data: SpiderMetadataToDisk,
    /// describes data processing, if any is needed
    pub data_processing: DataProcessDirectiveToDisk<DataProcess>,
}

impl TryFrom<Pipeline> for PipelineToDisk {
    type Error = anyhow::Error;

    fn try_from(pipeline: Pipeline) -> Result<Self, Self::Error> {
        let origin_data = pipeline.origin_data.as_ref().try_into()?;
        let data_processing = pipeline.data_processing.try_into()?;
        Ok(PipelineToDisk {
            origin_data,
            data_processing,
        })
    }
}
