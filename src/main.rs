#![deny(unused_crate_dependencies)]

mod args;
mod encryption_writer;
mod fs_carfiler;
mod fs_compression_encryption;
mod fs_copy;
mod fs_partition;
mod fsutil;
mod hasher;

use crate::fs_copy::copy_file_or_dir;
use clap::Parser;
use futures::{FutureExt, StreamExt};
use jwalk::WalkDirGeneric;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::StreamMap;

// Final output of the program
// TODO: this is a placeholder, we'll want to make this more useful later by using it to populate data
struct FinalMetadata {
    pub(crate) original_prefix_to_final_prefix: Vec<(PathBuf, PathBuf)>,
}

/* General Project Chores */
// TODO : Handle panics appropriately
// TODO : get rid of all the clones and stop copying around pathbufs
// TODO (laudiacay) : Handle pinning threads to CPU cores (with tokio localsets and runtimes?) correctly so that disk throughput is maximized

/* Dataprep:
 * 1. Copy files to scratch space from `input` directories to 'output-dir' directory
 * 2. Partition files into chunks of max size `target-chunk-size`
 * 3. Compress and encrypt each chunk in place. These chunks should be randomly named.
 * 4. TODO: Write out a manifest file that maps:
 *      - original file path to random chunk name / path
 *      - random chunk paths point to the key-path used to encrypt the chunk.
 *      - keys stored in csv file
 * 5. TODO: Encyprpt the manifest file in place with some master key.
 */
#[tokio::main]
async fn main() {
    // Parse command line arguments. see args.rs
    let args = args::Args::parse();

    // Get the output DIR from the command line arguments
    let output_dir = args.output_dir.canonicalize().unwrap();
    fsutil::ensure_path_exists_and_is_empty_dir(&output_dir)
        .expect("output directory must exist and be empty");

    // Get the key output DIR from the command line arguments
    let keys_dir = args.keys_dir.canonicalize().unwrap();
    fsutil::ensure_path_exists_and_is_empty_dir(&keys_dir)
        .expect("keys directory must exist and be empty");

    // Copy all the files over to a scratch directory
    let scratch_dir = output_dir.join("scratch");
    std::fs::create_dir(&scratch_dir).expect("could not create scratch directory");

    // TODO: We need to change how we are finalizing the output of the program. For now keep this struct.
    let mut final_output = FinalMetadata {
        original_prefix_to_final_prefix: Vec::new(),
    };

    /* Copy all the files over to a scratch directory */

    // Declare a stream map that will hold all the futures for copying and operating on the files
    let mut map = StreamMap::new();
    // Iterate over all the input directories
    for path_root in args.input {
        // Canonicalize the path
        let path_root = path_root
            .canonicalize()
            .expect("could not canonicalize path");

        // TODO (laudiacay) : Is this necessary?
        // Generate a random prefix for input DIR
        let new_root = scratch_dir.join(format!("{:x}", rand::random::<u64>()));
        final_output
            .original_prefix_to_final_prefix
            .push((path_root.clone(), new_root.clone()));

        // TODO (laudiacay): Is this really necessary? Look into jwalk plz
        // Walk the contents of the input directory and copy them to the scratch directory
        // Tally up the size of all the files in the DIR and tag each with the size
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
        // TODO make sure handoff from jwalk to tokio is efficient
        // Hand of the iterator generated by WalkDirGeneric to tokio. This turns the iterator into a stream
        let directory_stream = tokio_stream::iter(walk_dir);

        map.insert((path_root, new_root), directory_stream);
    }

    /* Perform deduplication and partitioning on the files */

    // Initialize a struct to memoize the hashes of files
    let seen_hashes = Arc::new(RwLock::new(HashMap::new()));
    // Iterate over all the futures in the stream map.
    let copied =
        map.then(|((path_root, new_root), dir_entry)| {
            // Clone the references to the seen_hashes map
            let local_seen_hashes = seen_hashes.clone();
            // Move the dir_entry into the future and copy the file.
            async move {
                copy_file_or_dir(path_root, dir_entry.unwrap(), new_root, local_seen_hashes).await
            }
        });

    let partitioned = copied.then(|copy_metadata| {
        let copy_metadata = copy_metadata.expect("copy failed");
        fs_partition::partition_file(copy_metadata).map(|res| res.unwrap())
    });

    // TODO for now we are doing compression in place, per-file. we could get things smaller.
    let _compressed_and_encrypted = partitioned.then(|file_data| {
        fs_compression_encryption::compress_and_encrypt_file_in_place(file_data)
            .map(|res| res.unwrap())
    });

    // TODO you can do this in one read of the file. entire pipeline. i think
    // TODO next you will need to encrypt filenames and other metadata (how are you hiding directory structure?)

    // TODO then you will need to write the car file

    // TODO then you will need to write the index file
    // TODO then you will need to write "filesystem rehydration"
}
