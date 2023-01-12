mod fs_iterator;
mod args;
mod fsutil;

use clap::Parser;
use futures::stream::BoxStream;
use futures::StreamExt;
use std::path::{PathBuf};

//use iroh_car::{CarWriter};
//use iroh_unixfs::builder::Config;
//use iroh_unixfs::chunker::ChunkerConfig;

use anyhow::{Result};
use crate::fs_iterator::do_singlethreaded_test;

// TODO don't copy around pathbufs you utter trainwreck
// TODO make a nice generalized function. make an iterable trait i guess.
async fn copy_to_scratch_space<'a>(
    paths: Vec<PathBuf>,
    scratch_root: PathBuf,
    follow_symlinks: bool,
) -> BoxStream<'a, Result<PathBuf>> {
    async_stream::try_stream! {
        let mut todo_paths_with_roots: Vec<(PathBuf, PathBuf)> =
            paths.into_iter().map(|p: PathBuf| (p.to_path_buf(), scratch_root.clone())).collect::<Vec<(PathBuf, PathBuf)>>();
        while let Some((path, scratch_root)) = todo_paths_with_roots.pop() {
            let mut new_target : PathBuf = scratch_root.to_path_buf();
            new_target.push(path.file_name().unwrap());
            if path.is_dir() {
                tokio::fs::create_dir(&new_target).await?;
                let mut new_paths : Vec<(PathBuf, PathBuf)> = Vec::new();
                let mut folder_contents = tokio::fs::read_dir(path).await?;
                while let Some(entry) = folder_contents.next_entry().await? {
                    new_paths.push((entry.path(), new_target.clone()));
                }
                todo_paths_with_roots.extend(new_paths);
            } else {
                if path.is_symlink() && !follow_symlinks {
                    continue;
                }
                tokio::fs::copy(path, &new_target).await?;
            };
            yield new_target;
        }
    }.boxed()
    //     let args_input_clone = args.input.clone();
    //     let _copystream =
    //         copy_to_scratch_space(args_input_clone, scratch_dir, args.follow_symlinks).await;
}

#[tokio::main]
async fn main() {
    let args = args::Args::parse();

    // get output directory
    fsutil::ensure_path_exists_and_is_empty_dir(&args.output_dir)
        .expect("output directory must exist and be empty");

    // get keys directory
    //fsutil::ensure_path_exists_and_is_empty_dir(&args.keys_dir)
    //    .expect("keys directory must exist and be empty");

    // copy all the files over to an encrypted scratch directory
    let scratch_dir = args.output_dir.join("scratch");
    std::fs::create_dir(&scratch_dir).expect("could not create scratch directory");
    // copy from inputs to scratch dir
    do_singlethreaded_test(scratch_dir).await;
}
