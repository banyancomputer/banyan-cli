// use crate::{
//     types::{
//         spider::SpiderMetadata,
//         pack_plan::PackPlan,
//     },
//     utils::hasher,
// };
// use anyhow::Result;
// use std::{
//     collections::HashMap,
//     path::PathBuf,
//     sync::{Arc, RwLock},
// };
//
// /// Copy a file or directory from one location to another. If the file is a duplicate, it will not be copied.
// ///
// /// # Arguments
// /// original_root: The root path of the original file or directory
// /// original_location: The path of the original file or directory within the root
// /// to_root: The root path to which the file or directory will be copied
// /// seen_hashes: A hashmap of blake2 hashes. Used to determine if a file is a duplicate or not.
// ///
// /// # Returns
// /// CopyMetadata struct that contains the original and new location of the file, as well as the blake2 hash of the file.
// // TODO (laudiacay): one day, do we use Rabin partitioning?
// pub fn plan_copy(
//     origin_data: SpiderMetadata,
//     to_root: PathBuf,
//     // This data structure is a little uglier than i would like it to be
//     seen_hashes: Arc<RwLock<HashMap<String, DuplicationMetadata>>>,
//     target_chunk_size: u64,
// ) -> Result<PackPlan> {
//     // If this is a directory
//     if origin_data.original_metadata.is_dir() {
//         // Return
//         Ok(PipelinePlan {
//             origin_data: Arc::new(origin_data),
//             data_processing: DataProcessDirective::Directory,
//         })
//     }
//     // If this is a symlink
//     else if origin_data.original_metadata.is_symlink() {
//         // return
//         Ok(PipelinePlan {
//             origin_data: Arc::new(origin_data),
//             data_processing: DataProcessDirective::Symlink,
//         })
//     }
//     // If this is a file
//     else {
//         // Compute the file hash
//         let file_hash = hasher::hash_file(&origin_data.canonicalized_path)?;
//         // Determine whether or not the file is a duplicate based on this hash
//         let duplicate_metadata = {
//             // grab a read lock and check if we've seen it //TODO claudia yikes
//             let seen_hashes = seen_hashes.read().unwrap();
//             seen_hashes.get(&file_hash).cloned()
//         };
//         // If there are file names associated with this hash, we've seen it before
//         let file_is_duplicate = duplicate_metadata.is_some();
//
//         // Enclose origin data in Reference Counter
//         let od = Arc::new(origin_data);
//
//         // Determine file size based on metadata
//         let file_size = od.original_metadata.len();
//         // Determine number of chunks required to store this file
//         let num_chunks = (file_size as f64 / target_chunk_size as f64).ceil() as u64;
//
//         // Compression and partition plans are the same even in the duplicate case
//         let compression_plan = CompressionPlan::new_zstd();
//         let partition_plan = PartitionPlan::new(target_chunk_size);
//         let encryption_plan = EncryptionPlan::new();
//
//         // Create a DataProcessPlan based on duplication status
//         let process_plan = if file_is_duplicate {
//             // DataProcessPlan for a duplicate file
//             DataProcessPlan {
//                 compression: compression_plan,
//                 partition: partition_plan,
//                 encryption: encryption_plan,
//                 writeout: WriteoutPlan::new(),
//                 duplication: DuplicationPlan {
//                     // This represents the location that the unpacker will extract the file to
//                     expected_location: Some(od.original_location.to_path_buf()),
//                 },
//             }
//         } else {
//             // DataProcessPlan for a new file
//             DataProcessPlan {
//                 compression: compression_plan,
//                 partition: partition_plan,
//                 encryption: encryption_plan,
//                 writeout: WriteoutPlan::new(),
//                 // There is no deuplication plan for new files
//                 duplication: DuplicationPlan::none(),
//             }
//         };
//
//         // Create a PipelinePlan
//         let pipeline_plan = PipelinePlan {
//             origin_data: od,
//             data_processing: DataProcessDirective::File(process_plan.clone()),
//         };
//
//         {
//             // Grab write lock TODO claudia come on
//             let mut seen_hashes = seen_hashes.write().unwrap();
//             // Insert the file hash and DuplicationMetadata into the HashMap
//             seen_hashes.insert(file_hash, process_plan.into());
//         }
//
//         // Now that plan is created and file is marked as seen, return Ok
//         Ok(pipeline_plan)
//     }
// }
//
// // TODO (thea-exe): Our inline tests
// #[cfg(test)]
// mod test {}
