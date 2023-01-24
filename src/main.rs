#![feature(io_error_more)]
#![feature(buf_read_has_data_left)]
#![deny(unused_crate_dependencies)]

mod args;
mod compression_tools;
mod crypto_tools;
mod fs_carfiler;
mod fsutil;
mod plan_copy;
mod spider;
mod types;
mod vacuum;

use crate::plan_copy::plan_copy;
use clap::Parser;
use futures::FutureExt;
use std::collections::HashMap;
use tokio_stream::StreamExt;

use std::sync::Arc;
use tokio::sync::RwLock;

/* General Project Chores */
// TODO (xBalbinus & thea-exe): Handle panics appropriately/get rid of all the unwraps
// TODO (xBalbinus & thea-exe): get rid of all the clones and stop copying around pathbufs
// TODO (xBalbinus & thea-exe): generally clean up imports and naming. the fs_yadayadayada stuff is particularly bad.

/* Hardcore project TODOs before mvp */
// TODO (laudiacay): We can implement the pipeline with a single FS read maybe. Look into this. Be sure to tally up the reads before attempting this.
// TODO (laudiacay): Encrypt filenames and other metadata. Need to hide directory structure.

/* Speculative Lifts */
// TODO (laudiacay): Can / Should we include an option to pack chunks into a CAR file? Look into this.
// TODO (laudiacay): What if we tried encrypting the file in place with one file handle. Look into this.
// TODO (laudiacay) : Handle pinning threads to CPU cores (with tokio localsets and runtimes?) correctly so that disk throughput is maximized

/* Dataprep:
 * 1. Copy files to scratch space from `input` directories to 'output-dir' directory
 * 2. Partition files into chunks of max size `target-chunk-size`
 * 3. Compress and encrypt each chunk in place. These chunks should be randomly named.
 * 4. TODO (laudiacay) : Write out a manifest file that maps:
 *      - original file path to random chunk name / path
 *      - random chunk paths point to the key-path used to encrypt the chunk.
 *      - keys stored in csv file
 * 5. TODO (laudiacay): Encyprpt the manifest file in place with some master key.
 * 6. TODO (amiller68 & laudiacay): Use manifest file to repopulate the original directory structure
 */
#[tokio::main]
async fn main() {
    // Parse command line arguments. see args.rs
    let args = args::Args::parse();

    // Get the output DIR from the command line arguments
    let output_dir = args.output_dir.canonicalize().unwrap();
    fsutil::ensure_path_exists_and_is_empty_dir(&output_dir)
        .expect("output directory must exist and be empty");

    // Note (amiller68): We don't necessarily need to create the keys dir, removing for now.
    // // Get the key output DIR from the command line arguments
    // let keys_dir = args.keys_dir.canonicalize().unwrap();
    // fsutil::ensure_path_exists_and_is_empty_dir(&keys_dir)
    //     .expect("keys directory must exist and be empty");

    // TODO: We need to change how we are finalizing the output of the program. For now keep this struct.
    // let mut final_output = FinalMetadata {
    //     original_prefix_to_final_prefix: Vec::new(),
    // };

    /* Copy all the files over to a scratch directory */

    println!("Walking input directories...");
    let spidered = spider::spider(args.input_dir, args.follow_links).unwrap();

    /* Perform deduplication and partitioning on the files */

    println!("De-duplicating and proposing partitions for files...");

    // Initialize a struct to memoize the hashes of files
    let seen_hashes = Arc::new(RwLock::new(HashMap::new()));
    // Iterate over all the futures in the stream map.
    let copy_plan = spidered.then(|origin_data| {
        let origin_data = origin_data.unwrap();
        let output_dir = output_dir.clone();
        // Clone the references to the seen_hashes map
        let local_seen_hashes = seen_hashes.clone();
        // Move the dir_entry into the future and copy the file.
        async move {
            plan_copy(
                origin_data,
                output_dir,
                local_seen_hashes,
                args.target_chunk_size,
            )
            .await
            .expect("copy failed")
        }
    });

    println!("Copying, compressing, encrypting, and writing to new FS...");

    // TODO (laudiacay): For now we are doing compression in place, per-file. Make this better.
    let _copied =
        copy_plan.then(|copy_plan| vacuum::pack::do_file_pipeline(copy_plan).map(|e| e.unwrap()));

    println!("Writing metadata...");

    // TODO (laudiacay): Write out a manifest file that maps: all the things needed to reconstruct the directory
    // For now just write out the content of compressed_and_encrypted to stdout
    //let _manifest = compressed_and_encrypted.for_each(|file_data| {
    //    println!("{file_data:?}");
    //    future::ready(())
    //});
}
