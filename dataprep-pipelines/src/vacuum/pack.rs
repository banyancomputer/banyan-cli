use anyhow::{anyhow, Result};
use flate2::bufread::GzEncoder;

use crate::cargovroom;
use crate::types::pipeline::{
    CompressionMetadata, DataProcess, EncryptionMetadata, EncryptionPart, Pipeline,
    WriteoutMetadata,
};
use crate::types::plan::{DataProcessPlan, PipelinePlan};
use crate::types::shared::DataProcessDirective;
use std::io::{BufRead, Cursor, Read, Write};
use std::sync::Arc;
use tokio::fs::File;
use tokio::sync::RwLock;

pub async fn do_file_pipeline(
    PipelinePlan {
        origin_data,
        data_processing,
    }: PipelinePlan,
    cars_writer: Arc<RwLock<cargovroom::CarsWriter<File>>>,
) -> Result<Pipeline> {
    match data_processing {
        DataProcessDirective::File(DataProcessPlan {
            compression,
            partition,
            encryption,
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

            let mut buf = [0; 1024 * 1024]; // 1MB buffer
            let mut encrypted_buf = Vec::new();

            // output
            let mut encrypted_pieces = Vec::new();
            let mut writeout_pieces = Vec::new();
            // iterate over the file, partitioning it and encrypting it
            while old_file_reader.has_data_left()? {
                // read a chunk of the file
                old_file_reader.read_exact(&mut buf)?;

                // make the encryptor
                let mut cursor = Cursor::new(&mut encrypted_buf);
                let mut new_file_encryptor = age::Encryptor::with_recipients(vec![Box::new(
                    encryption.identity.to_public(),
                )])
                .expect("could not create encryptor")
                .wrap_output(&mut cursor)?;

                // encrypt
                new_file_encryptor.write_all(&buf)?;
                new_file_encryptor.finish()?;

                // TODO this is a massive global lock on how many things can write out to the car files at once. this is not fantastic and should be fixed.
                let cars_location = cars_writer
                    .write()
                    .await
                    .write_block_raw(&encrypted_buf)
                    .await?;

                // write out the metadata
                writeout_pieces.push(cars_location);

                encrypted_pieces.push(EncryptionPart {
                    identity: encryption.identity.clone(),
                });
            }
            let encryption = EncryptionMetadata { encrypted_pieces };
            let compression = CompressionMetadata {
                compression_info: "GZIP".to_string(),
                size_after: 0, // TODO (laudiacay) figure out how to get this
            };
            let partition = partition.0;
            let writeout = WriteoutMetadata {
                car_locations: writeout_pieces,
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
// TODO (thea-exe): Our inline tests
#[cfg(test)]
mod test {}
