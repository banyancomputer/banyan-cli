#![feature(io_error_more)]
#![deny(unused_crate_dependencies)]

mod args;
mod compression_writer;
mod encryption_writer;
mod fs_carfiler;
mod fs_compression_encryption;
mod fs_copy;
mod fsutil;
mod hasher;
mod partition_reader;

use crate::fs_copy::prep_for_copy;
use clap::Parser;
//use futures::{FutureExt, StreamExt};
use futures::StreamExt;
use jwalk::WalkDirGeneric;
use std::cmp::Ordering;
use std::collections::HashMap;

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::StreamMap;

// Final output of the program
// TODO: Removed this for now, but keeping the code around
// struct FinalMetadata {
//     pub(crate) original_prefix_to_final_prefix: Vec<(PathBuf, PathBuf)>,
// }

/* General Project Chores */
// TODO (xBalbinus & thea-exe): Handle panics appropriately
// TODO (xBalbinus & thea-exe): get rid of all the clones and stop copying around pathbufs
// TODO (xBalbinus & thea-exe): get rid of all the unwraps
// TODO (xBalbinus & thea-exe): get rid of #derive(Debug) on all structs and instead implement a way to write results out. Reliant on having a solution for writing manifest files out.

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

    // Copy all the files over to a scratch directory
    let scratch_dir = output_dir.join("scratch");
    std::fs::create_dir(&scratch_dir).expect("could not create scratch directory");

    // TODO: We need to change how we are finalizing the output of the program. For now keep this struct.
    // let mut final_output = FinalMetadata {
    //     original_prefix_to_final_prefix: Vec::new(),
    // };

    /* Copy all the files over to a scratch directory */

    println!("Walking input directories...");

    // Declare a stream map that will hold all the futures for copying and operating on the files
    let mut map = StreamMap::new();
    // Iterate over all the input directories
    for path_root in args.input_dirs {
        // Canonicalize the path
        let path_root = path_root
            .canonicalize()
            .expect("could not canonicalize path");

        // TODO (laudiacay) : Is this necessary?
        // Generate a random prefix for input DIR
        let new_root = scratch_dir.join(format!("{:x}", rand::random::<u64>()));
        // Add the mapping from the original prefix to the new prefix to the final output
        // TODO: We need to change how we are finalizing the output of the program. For now keep this struct.
        // final_output
        //     .original_prefix_to_final_prefix
        //     .push((path_root.clone(), new_root.clone()));

        // TODO (laudiacay): Is this really necessary? Look into jwalk plz
        // Walk the contents of the input directory and copy them to the scratch directory
        let walk_dir = WalkDirGeneric::<(u64, Option<u64>)>::new(path_root.clone())
            // Only follow symlinks if the user specified it
            .follow_links(args.follow_links)
            // Process the contents of the directory in parallel
            .process_read_dir(|_depth, _path, read_dir_state, children| {
                // Read the first child of the directory
                if let Some(Ok(dir_entry)) = children.first_mut() {
                    // If this is a file than tally the size of the file to the read_dir_state
                    if dir_entry.file_type().is_file() {
                        // Get the size of the file
                        let file_size = dir_entry.metadata().unwrap().len();
                        // Add the size of the file to the read_dir_state
                        *read_dir_state += file_size;
                        // Not sure what this does
                        dir_entry.client_state = Some(file_size);
                    }
                };
                // Sort the children of the directory by size
                children.sort_by(|a, b| match (a, b) {
                    (Ok(a), Ok(b)) => a.file_name.cmp(&b.file_name),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => Ordering::Equal,
                });
            });
        // TODO (laudiacay): make sure handoff from jwalk to tokio is efficient
        // Hand of the iterator generated by WalkDirGeneric to tokio. This turns the iterator into a stream
        let directory_stream = tokio_stream::iter(walk_dir);
        // Insert the stream into the stream map
        map.insert((path_root, new_root), directory_stream);
    }

    /* Perform deduplication and partitioning on the files */

    println!("De-duplicating and proposing partitions for files...");

    // Initialize a struct to memoize the hashes of files
    let seen_hashes = Arc::new(RwLock::new(HashMap::new()));
    // Iterate over all the futures in the stream map.
    let copied = map.then(|((path_root, new_root), dir_entry)| {
        // Clone the references to the seen_hashes map
        let local_seen_hashes = seen_hashes.clone();
        // Move the dir_entry into the future and copy the file.
        async move {
            prep_for_copy(
                path_root,
                dir_entry.unwrap(),
                new_root,
                local_seen_hashes,
                args.target_chunk_size,
            )
            .await
            .expect("copy failed")
        }
    });

    println!("Partitioning files into chunks...");

    // Partition the files into chunks of size `target-chunk-size`
    //let partitioned = copied.then(|copy_metadata| {
    //fs_partition::partition_file(copy_metadata, args.target_chunk_size).map(|res| res.unwrap())
    //});

    println!("Compressing files and encrypting chunks...");

    // Compress and encrypt each chunk in place. These chunks should be randomly named.
    // TODO (laudiacay): For now we are doing compression in place, per-file. Make this better.
    //let compressed_and_encrypted = partitioned.then(|file_data| {
    //     fs_compression_encryption::compress_and_encrypt_partitioned_file(file_data)
    //         .map(|res| res.unwrap())
    // });

    println!("Writing metadata...");

    // TODO (laudiacay): Write out a manifest file that maps: all the things needed to reconstruct the directory
    // For now just write out the content of compressed_and_encrypted to stdout
    //let _manifest = compressed_and_encrypted.for_each(|file_data| {
    //    println!("{file_data:?}");
    //    future::ready(())
    //});
}
