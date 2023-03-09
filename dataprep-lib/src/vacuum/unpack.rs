use age::Decryptor;
use anyhow::{anyhow, Ok, Result};
use printio as _;
use std::{fs::File, io::BufReader, iter, path::Path};

use crate::types::unpack_plan::{UnpackPipelinePlan, UnpackPlan, UnpackType};

/// Unpack a single file, directory, or symlink using an UnpackPipelinePlan and output directory.
/// # Arguments
/// * `UnpackPipelinePlan` - Specifies where to find and how to unpack the data requested.
/// * `output_dir` - Specifies where to write the unpacked data.
/// # Returns
/// A `Result`, which can either succeed or fail. If it succeeds, it returns nothing. If it fails, it returns an error.
pub async fn do_unpack_pipeline(
    UnpackPipelinePlan {
        origin_data,
        data_processing,
    }: UnpackPipelinePlan,
    output_dir: &Path,
) -> Result<()> {
    // Construct the full relative output path by appending the subdirectory
    let output_path = output_dir.join(origin_data.original_location);

    // Processing directives require different handling
    match data_processing {
        UnpackType::File(UnpackPlan {
            compression,
            partition: _partition,
            encryption,
            writeout,
        }) => {
            // If the file already exists, skip it- we've already processed it
            if Path::exists(&output_path) {
                // TODO make this a warning
                println!("File already exists: {}", output_path.display());
                return Ok(());
            }

            // Create directories so that writing can take place
            std::fs::create_dir_all(output_path.parent().unwrap())?;

            // Otherwise make it
            let new_file_writer = File::create(output_path)?;

            // Ensure that our compression scheme is congruent with expectations
            // TODO use fancy .get_decoder() method :3
            assert_eq!(compression.compression_info, "ZSTD");

            // TODO (organizedgrime): switch back to iterating over chunks if use case arises
            // If there are chunks in the partition to process
            for chunk in writeout.chunk_locations.iter() {
                // Ensure that there is only one chunk
                // assert_eq!(partition.num_chunks, 1);
                // Chunk is a constant for now

                // Finish constructing the old file reader
                let old_file_reader = BufReader::new(File::open(chunk)?);

                // TODO naughty clone
                // Construct the old file reader by decrypting the encrypted piece
                let old_file_reader = {
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
                        &encryption.identity.clone() as &dyn age::Identity
                    ))?
                };

                // Copy the contents of the old reader into the new writer
                compression.decode(old_file_reader, &new_file_writer)?
                // TODO check the encryption tag at the end of the file?
            }

            // Return OK status
            Ok(())
        }
        UnpackType::Directory => {
            // TODO (laudiacay) set all the permissions and stuff right?
            tokio::fs::create_dir_all(&output_path)
                .await
                .map_err(|e| e.into())
        }
        UnpackType::Symlink(to) => {
            // TODO (laudiacay) set all the permissions and stuff right?
            tokio::fs::symlink(output_path, to)
                .await
                .map_err(|e| e.into())
        }
    }
}

// TODO (thea-exe): Our inline tests
// Note (amiller68): Testing may rely on decrypting the file, which is not yet implemented
#[cfg(test)]
mod test {}
