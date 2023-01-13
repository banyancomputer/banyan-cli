mod args;
mod fs_iterator;
mod fsencryption;
mod fsutil;

use clap::Parser;
use futures::FutureExt;
use tokio_stream::{StreamExt, StreamMap};

//use iroh_car::{CarWriter};
//use iroh_unixfs::builder::Config;
//use iroh_unixfs::chunker::ChunkerConfig;

//use crate::fs_iterator::do_singlethreaded_test;

#[tokio::main]
async fn main() {
    let args = args::Args::parse();

    // get output directory
    fsutil::ensure_path_exists_and_is_empty_dir(&args.output_dir)
        .expect("output directory must exist and be empty");

    // get keys directory
    fsutil::ensure_path_exists_and_is_empty_dir(&args.keys_dir)
        .expect("keys directory must exist and be empty");

    // copy all the files over to an encrypted scratch directory
    let scratch_dir = args.output_dir.join("scratch");
    std::fs::create_dir(&scratch_dir).expect("could not create scratch directory");

    // copy from inputs to scratch dir
    let copy_stream =
        fsutil::copy_paths_recursively(args.input, scratch_dir.clone(), args.follow_symlinks).await;

    let partition_stream = copy_stream
        .then(|(_, file)| {
            let file = file.unwrap();
            fsencryption::partition_file(file).map(|res| res.unwrap())
        });

    let encryption_metadata = partition_stream.then(|file_data| fsencryption::encrypt_file_in_place(file_data).map(|res| res.unwrap()));
}
