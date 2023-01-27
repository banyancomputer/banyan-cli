use crate::crypto_tools::encryption_writer::EncryptionWriter;
use aead::OsRng;
use anyhow::Result;
use flate2::bufread::GzEncoder;
use rand::{Rng, RngCore};
use tokio::fs::File;

use crate::types::pipeline::{CompressionMetadata, DataProcess, EncryptionMetadata, EncryptionPart, Pipeline, PipelineToDisk, WriteoutMetadata};
use crate::types::plan::{DataProcessPlan, PipelinePlan};
use crate::types::shared::{DataProcessDirective, DataProcessDirectiveToDisk};
use std::io::{BufRead, Read};
use std::path::PathBuf;
use flate2::read::GzDecoder;
use crate::crypto_tools::decryption_reader::DecryptionReader;

pub(crate) async fn do_file_pipeline(
    PipelineToDisk{
        origin_data,
        data_processing,
    }: PipelineToDisk,
    output_root: PathBuf,
) -> Result<()> {
    match data_processing {
        DataProcessDirectiveToDisk::File(DataProcessPlanToDisk {
                                             compression,
                                             partition,
                                             encryption,
                                             writeout,
                                         }) => {
            // TODO (laudiacay) async these reads. also is this buf setup right

            let mut new_file_writer = File::create(output_root.join(origin_data.original_location)).await?;

            for chunk in 0..partition.0.num_chunks {
                // open a reader to the original file
                let old_file_reader =
                    std::io::BufReader::new(std::fs::File::open(&writeout.chunk_locations.get(chunk))?);
                // put a gzip encoder on it then buffer it
                assert_eq!(compression.compression_info, "GZIP");
                let mut old_file_reader = std::io::BufReader::new(GzDecoder::new(
                    old_file_reader,
                ));
                let mut old_file_reader = DecryptionReader::new(old_file_reader, encryption.key).await;

                io.copy(&mut old_file_reader, &mut new_file_writer).await?;
                old_file_reader.finish()?;
                // TODO check the encryption tag at the end of the file
            }
            Ok(())
        }
        DataProcessDirectiveToDisk::Directory => {
            let loc = output_root.join(origin_data.original_location);
            // TODO (laudiacay) set all the permissions and stuff right?
            tokio::fs::create_dir_all(&loc).await.map_err(|e| e.into())
        },
        DataProcessDirectiveToDisk::Symlink => {
            let loc = output_root.join(origin_data.original_location);
            // TODO (laudiacay) set all the permissions and stuff right?
            tokio::fs::create_dir_all(&loc).await.map_err(|e| e.into())
        },
        DataProcessDirectiveToDisk::Duplicate(_smtd) => {
            todo!("hold off on duplicates for now");
        }
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
