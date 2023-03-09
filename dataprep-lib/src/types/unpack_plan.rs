use crate::types::{
    pack_plan::PackPipelinePlan,
    shared::{CompressionScheme, EncryptionScheme, PartitionScheme},
    spider::CodableSpiderMetadata,
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Metadata that is emitted on successful write into new filesystem
pub struct WriteoutLocations {
    /// mapping from compressed and encrypted chunks to their new locations
    pub chunk_locations: Vec<PathBuf>,
}

/// this struct is the completed data processing steps for a file and instructions for unpacking
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct UnpackPlan {
    /// describes how we compressed the entire file
    pub compression: CompressionScheme,
    /// describes how we partitioned the file into chunks (each slice is encrypted and written out
    /// separately- should be ENCRYPTION_TAG_SIZE bytes less than target_chunk_size!)
    pub partition: PartitionScheme,
    /// describes how we encrypted the file
    pub encryption: EncryptionScheme,
    /// describes how/where we wrote the file out on the new filesystem
    pub writeout: WriteoutLocations,
}

// TODO i have questions about this
// // all these are no-ops except for the File case
// impl TryFrom<DataProcessDirective<DataProcessPlan>> for DataProcessDirective<DataProcess> {
//     type Error = anyhow::Error;
//
//     fn try_from(
//         data_process_directive: DataProcessDirective<DataProcessPlan>,
//     ) -> Result<Self, Self::Error> {
//         match data_process_directive {
//             DataProcessDirective::File(process_plan) => {
//                 if process_plan.duplication.expected_location.is_some() {
//                     Ok(DataProcessDirective::File(process_plan.into()))
//                 } else {
//                     Err(Infallible::from(anyhow!("You have to process non-duplicate files!")))
//                 }
//             }
//             DataProcessDirective::Directory => Ok(DataProcessDirective::Directory),
//             DataProcessDirective::Symlink => Ok(DataProcessDirective::Symlink),
//         }
//     }
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The ways in which a packed file can be unpacked
pub enum UnpackType {
    /// Unpack a directory
    Directory,
    /// Unpack a symlink
    Symlink(PathBuf),
    /// Unpack a file
    File(UnpackPlan),
}

/// Describes how to unpack a file back to its origin.
/// A vector of structs with this type is encoded into the manifest file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnpackPipelinePlan {
    /// Describes where a SINGLE file came from on the original filesystem
    pub origin_data: CodableSpiderMetadata,
    /// Describes data processing, if any is needed
    pub data_processing: UnpackType,
}

impl TryFrom<PackPipelinePlan> for UnpackPipelinePlan {
    type Error = anyhow::Error;

    fn try_from(value: PackPipelinePlan) -> Result<Self, Self::Error> {
        match value {
            PackPipelinePlan::Directory(sm) => Ok(UnpackPipelinePlan {
                origin_data: (sm.as_ref()).try_into()?,
                data_processing: UnpackType::Directory,
            }),
            PackPipelinePlan::Symlink(sm, loc) => Ok(UnpackPipelinePlan {
                origin_data: (sm.as_ref()).try_into()?,
                data_processing: UnpackType::Symlink(loc),
            }),
            _ => Err(anyhow!("You have to go process non-duplicate files!")),
        }
    }
}

/// This is the struct that becomes the contents of the manifest file.
/// It may seem silly to have a struct that has only one field, but in
/// versioning this struct, we can also version its children identically.
/// As well as any other fields we may add / remove in the future.
#[derive(Debug, Serialize, Deserialize)]
pub struct ManifestData {
    /// The project version that was used to encode this ManifestData
    pub version: String,
    /// Specification for how to unpack files back to their original locations
    pub unpack_plans: Vec<UnpackPipelinePlan>,
}
