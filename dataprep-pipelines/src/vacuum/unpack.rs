use age::Decryptor;
use anyhow::{anyhow, Result};
use flate2::write::GzDecoder;
use printio as _;
use std::fs::File;
use std::io::BufReader;
use std::iter;
use std::path::PathBuf;

use crate::types::pipeline::{DataProcess, PipelineToDisk};
use crate::types::shared::DataProcessDirectiveToDisk;

// Unpack a single file, directory, or symlink
pub async fn do_file_pipeline(
    PipelineToDisk {
        origin_data,
        data_processing,
    }: PipelineToDisk,
    input_dir: PathBuf,
    output_dir: PathBuf,
) -> Result<()> {
    // Processing directives require different handling
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
            // Ensure that our compression scheme is congruent with expectations
            assert_eq!(compression.compression_info, "GZIP");
            // Create a new file writer
            let mut new_file_writer = GzDecoder::new(new_file_writer);

            // For each chunk in the partition
            for chunk in 0..partition.num_chunks {
                //TODO: (organizedgrime) if we're ensuring that there is only one chunk here,
                // why do we need to iterate over all the chunks? might as create an
                // if block for partition.num_chunks > 0, run this assertion, and treat chunk as a constant.
                assert_eq!(partition.num_chunks, 1);

                // Construct the file path within the input directory
                let subpath = writeout.chunk_locations.get(chunk as usize).ok_or(anyhow!(
                    "could not find the chunk location for chunk {}!",
                    chunk
                ))?;

                // Finish constructing the old file reader
                let old_file_reader = BufReader::new(File::open(input_dir.join(subpath))?);

                // Find the encrypted piece for this chunk
                let encrypted_piece =
                    encryption
                        .encrypted_pieces
                        .get(chunk as usize)
                        .ok_or(anyhow!(
                            "could not find the encrypted piece for chunk {}!",
                            chunk
                        ))?;

                // TODO naughty clone
                // Construct the old file reader by decrypting the encrypted piece
                let mut old_file_reader = {
                    // Match decryptor type to ensure compatibility;
                    // use internal variable to construct the decryptor
                    let decryptor = match Decryptor::new(old_file_reader)? {
                        Decryptor::Recipients(decryptor) => decryptor,
                        Decryptor::Passphrase(_) => {
                            return Err(anyhow!("Passphrase decryption not supported"))
                        }
                    };

                    // Use the decryptor to decrypt the encrypted piece; return result
                    decryptor.decrypt(iter::once(
                        &encrypted_piece.identity.clone() as &dyn age::Identity
                    ))?
                };

                // Copy the contents of the old reader into the new writer
                std::io::copy(&mut old_file_reader, &mut new_file_writer)?;

                // old_file_reader.finish()?;
                // TODO check the encryption tag at the end of the file
            }

            // Return OK status
            Ok(())
        }
        DataProcessDirectiveToDisk::Duplicate(_smtd) => {
            todo!("Duplicates are not yet implemented. Come back later!");
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
    }
}
// TODO (thea-exe): Our inline tests
// Note (amiller68): Testing may rely on decrypting the file, which is not yet implemented
#[cfg(test)]
mod test {}
