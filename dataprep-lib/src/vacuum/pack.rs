use crate::types::{
    pack_plan::{PackPipelinePlan, PackPlan},
    unpack_plan::{UnpackPipelinePlan, UnpackPlan, UnpackType, WriteoutLocations},
};
use anyhow::{anyhow, Result};
use indicatif::ProgressBar;
use std::{
    fs::File,
    io::{BufReader, Read},
    sync::{Arc, Mutex},
};
use wnfs::common::{BlockStore, DiskBlockStore};

// TODO in the battle against repeated code... fn refresh_file_encryptor() ->
/// This function takes in a plan for how to process an individual file group, directory, or symlink,
/// and uses that plan to pack the data into the specified location.
/// # Arguments
/// * `pack_pipeline_plan` - The plan for how to pack the this individual file group, directory, or symlink.
/// # Returns
/// Returns a `Result<Vec<UnpackPipelinePlan>>`. Provides vector of plans for unpacking the newly created files in cases where
/// the file was chunked and compressed successfully, or an error if something went wrong.
pub async fn do_pack_pipeline(
    blockstore: &DiskBlockStore,
    pack_pipeline_plan: PackPipelinePlan,
    progress_bar: Arc<Mutex<ProgressBar>>,
) -> Result<Vec<UnpackPipelinePlan>> {
    match pack_pipeline_plan {
        // If this is a FileGroup
        PackPipelinePlan::FileGroup(metadatas, pack_plan) => {
            let PackPlan {
                compression,
                partition,
                encryption,
                size_in_bytes,
            } = pack_plan;
            // Open the original file (just the first one!)
            let file = File::open(&metadatas.get(0)
                .expect("why is there nothing in metadatas").canonicalized_path)
                .map_err(|e| anyhow!("could not find canonicalized path when trying to open reader to original file! {}", e))?;
            // Keep track of how many bytes we've not yet processed
            let mut remaining_bytes = file.metadata().unwrap().len();
            // Create a reader for the original file
            let old_file_reader = BufReader::new(file);
            // Keep track of the location of encrypted pieces
            let mut chunk_locations = Vec::new();

            // While we've not yet seeked through the entirety of the file
            while remaining_bytes > 0 {
                // Determine how much of the file we're going to read
                let read_size = std::cmp::min(partition.chunk_size, remaining_bytes);
                // Construct a reader that will prevent us from reading the entire file at once
                // TODO (organizedgrime) something about inner vs outer chunking?
                let chunk_reader = old_file_reader.get_ref().take(read_size);
                // Create a buffer to hold the compressed bytes
                let mut bytes: Vec<u8> = vec![];
                // Encode and compress the chunk
                compression.encode(chunk_reader, &mut bytes)?;
                // Put the bytes into the BlockStore and retrieve the associated CID
                let cid = blockstore.put_serializable(&bytes).await.unwrap();
                // Append the writeout location to the list of chunks
                chunk_locations.push(blockstore.path.join(cid.to_string()));
                // Determine how much of the file has yet to be written
                remaining_bytes -= read_size;
                // Denote progress
                progress_bar.lock().unwrap().inc(1);
            }

            // Create a new UnpackType::File with the chunk locations constructed earlier
            let unpack_file = UnpackType::File(UnpackPlan {
                compression,
                partition,
                encryption,
                writeout: WriteoutLocations { chunk_locations },
                size_in_bytes,
            });

            // Return okay status with all UnpackPipelinePlans
            Ok(
                // For each SpiderMetadata in the FileGroup (even duplicates)
                metadatas
                    .iter()
                    .map(|metadata| {
                        // Construct a new UnpackPipelinePlan
                        UnpackPipelinePlan {
                            // Despite being a try_into, this is guaranteed to succeed given the context of the function
                            origin_data: metadata.as_ref().try_into().unwrap(),
                            data_processing: unpack_file.clone(),
                        }
                    })
                    .collect::<Vec<UnpackPipelinePlan>>(),
            )
        }
        // If this is a directory or symlink
        d @ PackPipelinePlan::Directory(_) | d @ PackPipelinePlan::Symlink(..) => {
            // Directly convert into an UnpackPipelinePlan
            progress_bar.lock().unwrap().inc(1);
            Ok(vec![d.try_into()?])
        }
    }
}
// TODO (thea-exe): Our inline tests
#[cfg(test)]
mod test {}
