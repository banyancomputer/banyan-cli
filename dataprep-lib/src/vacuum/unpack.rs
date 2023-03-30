use age::{Decryptor, stream};
use anyhow::{anyhow, Ok, Result};
use std::{io::Write, borrow::{Borrow, Cow}, str::FromStr};
use wnfs::{common::{DiskBlockStore, BlockStore}, libipld::{IpldCodec, Cid, block}};
use std::{fs::File, io::{BufReader, BufWriter}, iter, path::Path, sync::Mutex, os::unix::prelude::FileExt};
use crate::types::unpack_plan::{UnpackPipelinePlan, UnpackPlan, UnpackType};
use indicatif::ProgressBar;
use std::sync::Arc;

/// Unpack a single file, directory, or symlink using an UnpackPipelinePlan and output directory.
/// # Arguments
/// * `UnpackPipelinePlan` - Specifies where to find and how to unpack the data requested.
/// * `output_dir` - Specifies where to write the unpacked data.
/// # Returns
/// A `Result`, which can either succeed or fail. If it succeeds, it returns nothing. If it fails, it returns an error.
pub async fn do_unpack_pipeline(
    blockstore: &DiskBlockStore,
    UnpackPipelinePlan {
        origin_data,
        data_processing,
    }: UnpackPipelinePlan,
    output_dir: &Path,
    progress_bar: Arc<Mutex<ProgressBar>>,
) -> Result<()> {
    // Construct the full relative output path by appending the subdirectory
    let output_path = output_dir.join(origin_data.original_location);

    // Processing directives require different handling
    match data_processing {
        UnpackType::File(UnpackPlan {
            compression,
            partition: _partition,
            encryption: _encryption,
            writeout,
            ..
        }) => {
            // If the file already exists, skip it- we've already processed it
            if Path::exists(&output_path) {
                // TODO make this a warning
                warn!("File already exists: {}", output_path.display());
                return Ok(());
            }

            // Create directories so that writing can take place
            std::fs::create_dir_all(
                output_path
                    .parent()
                    .expect("could not get parent directory of output file! {output_path}"),
            )
            .map_err(|e| anyhow!("could not create parent directory for output file! {}", e))?;

            // Otherwise make it
            let mut new_file_writer = BufWriter::new(File::create(output_path)
                .map_err(|e| anyhow!("could not create new file for writing! {}", e))?);

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
                let cid = Cid::from_str(chunk.file_name().unwrap().to_str().unwrap()).unwrap();
                // Grab the bytes associated with this CID
                let bytes: Vec<u8> = blockstore.get_deserializable(&cid).await.unwrap();
                // Write these bytes to the new file writer
                new_file_writer.write_all(&bytes).unwrap();
                // Flush the new file writer
                new_file_writer.flush().unwrap();
                // Update the progress bar
                progress_bar.lock().unwrap().inc(1);
            }
            // Return OK status
            Ok(())
        }
        UnpackType::Directory => {
            // TODO (laudiacay) set all the permissions and stuff right?
            let ret = tokio::fs::create_dir_all(&output_path)
                .await
                .map_err(|e| e.into());
            progress_bar.lock().unwrap().inc(1);
            ret
        }
        UnpackType::Symlink(to) => {
            // TODO (laudiacay) set all the permissions and stuff right?
            let ret = tokio::fs::symlink(output_path, to)
                .await
                .map_err(|e| e.into());
            progress_bar.lock().unwrap().inc(1);
            ret
        }
    }
}

// TODO (thea-exe): Our inline tests
// Note (amiller68): Testing may rely on decrypting the file, which is not yet implemented
#[cfg(test)]
mod test {}
