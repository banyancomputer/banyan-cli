use anyhow::{anyhow, Result};
use std::iter;

use crate::types::pipeline::{DataProcess, PipelineToDisk};
use crate::types::shared::DataProcessDirectiveToDisk;
use flate2::write::GzDecoder;
use printio as _;
use std::path::PathBuf;

pub async fn do_file_pipeline(
    PipelineToDisk {
        origin_data,
        data_processing,
    }: PipelineToDisk,
    input_dir: PathBuf,
    output_dir: PathBuf,
) -> Result<()> {
    match data_processing {
        DataProcessDirectiveToDisk::File(DataProcess {
            compression,
            partition,
            encryption,
            writeout,
        }) => {
            // TODO (laudiacay) async these reads. also is this buf setup right
            let output = output_dir.join(origin_data.original_location);
            let new_file_writer = std::fs::File::create(output)?;
            assert_eq!(compression.compression_info, "GZIP");
            let mut new_file_writer = GzDecoder::new(new_file_writer);

            for chunk in 0..partition.num_chunks {
                assert_eq!(partition.num_chunks, 1);
                // open a reader to the original file
                let old_file_reader = std::io::BufReader::new(std::fs::File::open(
                    input_dir.join(writeout.chunk_locations.get(chunk as usize).ok_or(anyhow!(
                        "could not find the chunk location for chunk {}!",
                        chunk
                    ))?),
                )?);

                let encrypted_piece =
                    encryption
                        .encrypted_pieces
                        .get(chunk as usize)
                        .ok_or(anyhow!(
                            "could not find the encrypted piece for chunk {}!",
                            chunk
                        ))?;
                // TODO naughty clone

                let mut old_file_reader = {
                    let decryptor = match age::Decryptor::new(old_file_reader)? {
                        age::Decryptor::Recipients(decryptor) => decryptor,
                        age::Decryptor::Passphrase(_) => {
                            return Err(anyhow!("passphrase decryption not supported"))
                        }
                    };
                    decryptor.decrypt(iter::once(
                        &encrypted_piece.identity.clone() as &dyn age::Identity
                    ))?
                };
                // put a gzip encoder on it then buffer it

                std::io::copy(&mut old_file_reader, &mut new_file_writer)?;
                // TODO check the encryption tag at the end of the file
                // old_file_reader.finish()?;
            }
            Ok(())
        }
        DataProcessDirectiveToDisk::Directory => {
            let loc = output_dir.join(origin_data.original_location);
            // TODO (laudiacay) set all the permissions and stuff right?
            tokio::fs::create_dir_all(&loc).await.map_err(|e| e.into())
        }
        DataProcessDirectiveToDisk::Symlink => {
            let loc = output_dir.join(origin_data.original_location);
            // TODO (laudiacay) set all the permissions and stuff right?
            tokio::fs::create_dir_all(&loc).await.map_err(|e| e.into())
        }
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
