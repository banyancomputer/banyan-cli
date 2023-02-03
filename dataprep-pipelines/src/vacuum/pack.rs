use anyhow::{anyhow, Result};
use flate2::bufread::GzEncoder;

use crate::types::pipeline::{
    CompressionMetadata, DataProcess, EncryptionMetadata, EncryptionPart, Pipeline,
    WriteoutMetadata,
};
use crate::types::plan::{DataProcessPlan, PipelinePlan};
use crate::types::shared::DataProcessDirective;
use std::io::{BufRead, Read};

pub async fn do_file_pipeline(
    PipelinePlan {
        origin_data,
        data_processing,
    }: PipelinePlan,
) -> Result<Pipeline> {
    match data_processing {
        DataProcessDirective::File(DataProcessPlan {
            compression,
            partition,
            encryption,
            writeout,
        }) => {
            // TODO (laudiacay) async these reads. also is this buf setup right

            // open a reader to the original file
            let old_file_reader =
                std::io::BufReader::new(std::fs::File::open(&origin_data.canonicalized_path).map_err(|e| anyhow!("could not find canonicalized path when trying to open reader to original file! {}",e))
                    ?);

            // put a gzip encoder on it then buffer it
            assert_eq!(compression.compression_info, "GZIP");
            let mut old_file_reader = std::io::BufReader::new(GzEncoder::new(
                old_file_reader,
                flate2::Compression::default(),
            ));

            // output
            let mut encrypted_pieces = Vec::new();
            let mut i = 0;
            // iterate over the file, partitioning it and encrypting it
            while old_file_reader.has_data_left()? {
                // read a chunk of the file
                // TODO (laudiacay) write down somewhere which bytes of the OG file this was.
                let mut old_file_take = old_file_reader.take(partition.0.chunk_size);
                // open the output file for writing
                let new_file_writer =
                    std::fs::File::create(&writeout.output_paths[i]).map_err(|e| {
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
            let encryption = EncryptionMetadata { encrypted_pieces };
            let compression = CompressionMetadata {
                compression_info: "GZIP".to_string(),
                size_after: 0, // TODO (laudiacay) figure out how to get this
            };
            let partition = partition.0;
            let writeout = WriteoutMetadata {
                chunk_locations: writeout.output_paths,
            };
            let data_processing = DataProcessDirective::File(DataProcess {
                encryption,
                compression,
                partition,
                writeout,
            });
            Ok(Pipeline {
                origin_data,
                data_processing,
            })
        }
        _ => Ok(Pipeline {
            origin_data,
            data_processing: data_processing.try_into()?,
        }),
    }
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
