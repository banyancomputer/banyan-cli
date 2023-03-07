use anyhow::{anyhow, Result};
use std::{
    fs::File,
    io::{BufReader, Read, Seek},
};

use crate::types::{
    pack_plan::{PackPipelinePlan, PackPlan},
    unpack_plan::{UnpackPipelinePlan, UnpackPlan, UnpackType, WriteoutLocations},
};

// TODO in the battle against repeated code... fn refresh_file_encryptor() ->

/// this file takes in a plan for how to process an identical file group, dir, or symlink,
/// and performs that action on the filesystem
/// returns a struct that can be used to unpack the file.
pub async fn do_file_pipeline(
    pack_pipeline_plan: PackPipelinePlan,
) -> Result<Vec<UnpackPipelinePlan>> {
    match pack_pipeline_plan {
        // If this is a file
        PackPipelinePlan::FileGroup(metadatas, pack_plan) => {
            let PackPlan {
                compression,
                partition,
                encryption,
                writeout,
            } = pack_plan;

            // TODO (organizedgrime) async these reads? also is this buf setup right?

            // Open the original file (just the first one!)
            let file = File::open(&metadatas.get(0)
                .expect("why is there nothing in metadatas").canonicalized_path)
                .map_err(|e| anyhow!("could not find canonicalized path when trying to open reader to original file! {}", e))?;
            // Create a reader for the original file
            let old_file_reader = BufReader::new(file);
            // Determine the length of that original file
            let file_len = old_file_reader.get_ref().seek(std::io::SeekFrom::End(0))?;

            // Keep track of the location of encrypted pieces
            let mut writeout_locations = Vec::new();

            // New packed file name
            let mut new_path: String;
            // Location of this new packed file is also dependent on the writeout location
            let mut new_file_loc: std::path::PathBuf;
            // Create a new file writer at this location
            let mut new_file_writer: File;
            // Create a new encryptor for this file
            let mut new_file_encryptor: Option<age::stream::StreamWriter<File>> = None;

            // Reset the file seeking position to the start of the file
            old_file_reader
                .get_ref()
                .seek(std::io::SeekFrom::Start(0))?;

            // Keep track of the data already written into this
            let mut remaining_bytes = file_len;

            // While we've not yet seeked through the entirety of the file
            while remaining_bytes > 0 {
                // Create a new file name and writeout location
                new_path = format!("{}{}", uuid::Uuid::new_v4(), ".packed");
                new_file_loc = writeout.join(new_path);

                // open the output file for writing
                new_file_writer = File::create(&new_file_loc)
                    .map_err(|e| anyhow!("could not create new file for writing! {}", e))?;

                // Create a new encryptor for this file
                new_file_encryptor = Some(
                    age::Encryptor::with_recipients(vec![Box::new(
                        encryption.identity.to_public(),
                    )])
                    .expect("could not create encryptor")
                    .wrap_output(new_file_writer)?,
                );

                // Append the writeout location
                writeout_locations.push(new_file_loc);

                // Determine how much of the file we're going to read
                let read_size = std::cmp::min(partition.chunk_size, remaining_bytes);

                // Construct a reader that will prevent us from reading the entire file at once
                // TODO (organizedgrime) something about inner vs outer chunking?
                let chunk_reader = old_file_reader.get_ref().take(read_size);

                // TODO (organizedgrime) maybe we can async these one day, a girl can dream
                // Encode and compress the chunk
                zstd::stream::copy_encode(chunk_reader, new_file_encryptor.as_mut().unwrap(), 1)?;

                // Determine how much of the file has yet to be written
                remaining_bytes -= read_size;

                // Close the previously written chunk
                new_file_encryptor
                    .unwrap()
                    .finish()
                    .map_err(|e| anyhow!("could not finish encryption! {}", e))?;
            }
            // TODO turn this into a map
            let mut ret = vec![];
            let dpp = UnpackType::File(UnpackPlan {
                compression,
                partition,
                encryption,
                writeout: WriteoutLocations {
                    chunk_locations: writeout_locations.clone(),
                },
            });

            // For each metadata
            for m in metadatas {
                // Construct a new UnpackPipelinePlan
                ret.push(UnpackPipelinePlan {
                    origin_data: m.as_ref().try_into()?,
                    data_processing: dpp.clone(),
                })
            }

            // Return okay status with all UnpackPipelinePlans
            Ok(ret)
        }
        // If this is a directory or symlink
        d @ PackPipelinePlan::Directory(_) | d @ PackPipelinePlan::Symlink(..) => {
            Ok(vec![d.try_into()?])
        }
    }
}
// TODO (thea-exe): Our inline tests
#[cfg(test)]
mod test {}
