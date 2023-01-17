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
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::PathBuf;

use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_stream::StreamMap;

//use iroh_car::{CarWriter};
//use iroh_unixfs::builder::Config;
//use iroh_unixfs::chunker::ChunkerConfig;

//use crate::fs_iterator::do_singlethreaded_test;

struct FinalMetadata {
    pub(crate) original_prefix_to_final_prefix: Vec<(PathBuf, PathBuf)>,
}
// TODO handle panics better
// TODO handle pinning (with tokio localsets and runtimes?) correctly so that disk throughput is maximized
// TODO get rid of all the clones and stop copying around pathbufs...
#[tokio::main]
async fn main() {
    let args = args::Args::parse();

    // get output directory
    let output_dir = args.output_dir.canonicalize().unwrap();
    fsutil::ensure_path_exists_and_is_empty_dir(&output_dir)
        .expect("output directory must exist and be empty");

    // get keys directory
    let keys_dir = args.keys_dir.canonicalize().unwrap();
    fsutil::ensure_path_exists_and_is_empty_dir(&keys_dir)
        .expect("keys directory must exist and be empty");

    // copy all the files over to an encrypted scratch directory
    let scratch_dir = output_dir.join("scratch");
    std::fs::create_dir(&scratch_dir).expect("could not create scratch directory");

    let mut final_output = FinalMetadata {
        original_prefix_to_final_prefix: Vec::new(),
    };

    use jwalk::WalkDirGeneric;

    let mut map = StreamMap::new();
    for path_root in args.input {
        // canonicalize the top of the path, whichever bizarro way they wrote it out
        let path_root = path_root
            .canonicalize()
            .expect("could not canonicalize path");

        // generate a random string to use as the new output root for this path_root
        let new_root = scratch_dir.join(format!("{:x}", rand::random::<u64>()));
        final_output
            .original_prefix_to_final_prefix
            .push((path_root.clone(), new_root.clone()));

        // walk the directory!
        let walk_dir = WalkDirGeneric::<(u64, Option<u64>)>::new(path_root.clone())
            .follow_links(args.follow_links)
            .process_read_dir(|_depth, _path, read_dir_state, children| {
                if let Some(Ok(dir_entry)) = children.first_mut() {
                    if dir_entry.file_type().is_file() {
                        // get file size
                        let file_size = dir_entry.metadata().unwrap().len();
                        *read_dir_state += file_size;
                        dir_entry.client_state = Some(file_size);
                    }
                };
                // sort 'em
                children.sort_by(|a, b| match (a, b) {
                    (Ok(a), Ok(b)) => a.file_name.cmp(&b.file_name),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => Ordering::Equal,
                });
            });
        // TODO make sure handoff from jwalk to tokio is efficient
        let directory_stream = tokio_stream::iter(walk_dir);
        map.insert((path_root, new_root), directory_stream);
    }
    // here, you should collect the directory streams really fast just so you can get it out of the way before the heavier work starts.
    // want this ready before car file creation :)

    let seen_hashes = Arc::new(RwLock::new(HashMap::new()));
    let copied =
        map.then(|((path_root, new_root), dir_entry)| {
            let local_seen_hashes = seen_hashes.clone();
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
