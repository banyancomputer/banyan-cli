use anyhow::{anyhow, Result};
use flate2::{bufread::GzEncoder, Compression};
use std::{fs::File, io::BufReader};

use crate::types::{
    pipeline::{EncryptionPart, Pipeline},
    plan::{DataProcessPlan, PipelinePlan},
    shared::DataProcessDirective,
};
use std::io::{BufRead, Read};

pub async fn do_file_pipeline(
    PipelinePlan {
        origin_data,
        data_processing,
    }: PipelinePlan,
) -> Result<Pipeline> {
    match data_processing.clone() {
        // If this is a file
        DataProcessDirective::File(dpp) => {
            let DataProcessPlan {
                compression,
                partition,
                encryption,
                writeout,
                duplication,
            } = dpp.clone();
            // TODO (laudiacay) async these reads. also is this buf setup right

            // If this is a duplicate file, we don't need to do anything
            if duplication.expected_location.is_some() {
                return Ok(Pipeline {
                    origin_data,
                    data_processing: data_processing.try_into()?,
                });
            }

            // open a reader to the original file
            let old_file_reader =
                BufReader::new(
                    File::open(&origin_data.canonicalized_path)
                        .map_err(|e| anyhow!("could not find canonicalized path when trying to open reader to original file! {}", e))
                ?);

            // put a gzip encoder on it then buffer it
            assert_eq!(compression.compression_info, "GZIP");

            let mut old_file_reader =
                BufReader::new(GzEncoder::new(old_file_reader, Compression::default()));

            // output
            let mut encrypted_pieces = Vec::new();
            let mut i = 0;
            // iterate over the file, partitioning it and encrypting it
            while old_file_reader.has_data_left()? {
                // read a chunk of the file
                // TODO (laudiacay) write down somewhere which bytes of the OG file this was.
                let mut old_file_take = old_file_reader.take(partition.0.chunk_size);
                // open the output file for writing
                let new_file_writer = File::create(&writeout.output_paths[i]).map_err(|e| {
                    anyhow!(
                        "could not create new file writer! {} at {:?}",
                        e,
                        &writeout.output_paths[i]
                    )
                })?;

                // make the encryptor
                let mut new_file_encryptor = age::Encryptor::with_recipients(vec![Box::new(
                    encryption.identity.to_public(),
                )])
                .expect("could not create encryptor")
                .wrap_output(new_file_writer)?;

                // TODO this blocks.  I don't know how to make it async
                // copy the data from the old file to the new file. also does the compression tag!
                std::io::copy(&mut old_file_take, &mut new_file_encryptor)
                    .map_err(|e| anyhow!("could not copy data from old file to new file! {}", e))?;

                old_file_reader = old_file_take.into_inner();

                // finish the encryption (write out the tag and anything in the buffer)
                new_file_encryptor
                    .finish()
                    .map_err(|e| anyhow!("could not finish encryption! {}", e))?;

                // write out the metadata
                encrypted_pieces.push(EncryptionPart {
                    identity: encryption.identity.clone(),
                });
                i += 1;
            }

            //
            Ok(Pipeline {
                origin_data,
                data_processing: DataProcessDirective::File(dpp.try_into()?),
            })
        }
        // If this is a directory, symlink, or duplicate
        _ => Ok(Pipeline {
            origin_data,
            data_processing: data_processing.try_into()?,
        }),
    }
}
// TODO (thea-exe): Our inline tests
#[cfg(test)]
mod test {}
