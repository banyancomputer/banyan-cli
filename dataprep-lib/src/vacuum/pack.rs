use anyhow::{anyhow, Result};
use std::{fs::File, io::BufReader};
use std::io::Seek;

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
            let file = File::open(&metadatas.get(0).expect("why is there nothing in metadatas").canonicalized_path)
                .map_err(|e| anyhow!("could not find canonicalized path when trying to open reader to original file! {}", e))?;

            // Build an encoder for the file
            // TODO one day make this look like compression_info.get_encoder
            

            // Keep track of encrypted pieces
            let mut writeout_locations = Vec::new();
            let mut current_chunk_num = 0;
            let mut current_chunk_size = 0;

            let mut new_path = uuid::Uuid::new_v4().to_string();
            new_path.push_str(".packed");
            let mut new_file_loc = writeout.join(new_path);
            let mut new_file_writer = File::create(&new_file_loc)
                .map_err(|e| anyhow!("could not create new file for writing! {}", e))?;
            let mut new_file_encryptor =
                age::Encryptor::with_recipients(vec![Box::new(encryption.identity.to_public())])
                    .expect("could not create encryptor")
                    .wrap_output(new_file_writer)?;
            writeout_locations.push(new_file_loc);


            let old_file_reader = BufReader::new(file);
            let file_len = old_file_reader.get_ref().seek(std::io::SeekFrom::End(0))?;
            // we aren't done with the file until we've read all of it
            while old_file_reader.get_ref().seek(std::io::SeekFrom::Current(0))? < file_len {
                if current_chunk_size >= partition.chunk_size {
                    current_chunk_num += 1;
                    current_chunk_size = 0;

                    // finish the encryption (write out the tag and anything in the buffer)
                    // TODO deduplicate code
                    new_file_encryptor
                        .finish()
                        .map_err(|e| anyhow!("could not finish encryption! {}", e))?;

                    new_path = uuid::Uuid::new_v4().to_string();
                    new_path.push_str(".packed");
                    new_file_loc = writeout.join(new_path);
                    // open the output file for writing
                    new_file_writer = File::create(&new_file_loc)
                        .map_err(|e| anyhow!("could not create new file for writing! {}", e))?;
                    new_file_encryptor = age::Encryptor::with_recipients(vec![Box::new(
                        encryption.identity.to_public(),
                    )])
                        .expect("could not create encryptor")
                        .wrap_output(new_file_writer)?;
                    writeout_locations.push(new_file_loc);
                }

                // read a chunk of the file
                // TODO (laudiacay) write down somewhere which bytes of the OG file this was.
                // let mut old_file_take =
                //     old_file_reader.take((partition.chunk_size - current_chunk_size) as usize);

                // TODO this blocks.  I don't know how to make it async
                // copy the data from the old file to the new file. also does the compression tag!

                zstd::stream::copy_encode(old_file_reader.get_ref(), &mut new_file_encryptor, 1);
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

            for m in metadatas {
                ret.push(UnpackPipelinePlan {
                    origin_data: m.as_ref().try_into()?,
                    data_processing: dpp.clone(),
                })
            }
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
