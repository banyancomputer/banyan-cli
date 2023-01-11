mod fs_iterator;

// import clap
use clap::Parser;
use futures::stream::BoxStream;
use futures::StreamExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

//use iroh_car::{CarWriter};
use iroh_unixfs::builder::Config;
use iroh_unixfs::chunker::ChunkerConfig;

use anyhow::{anyhow, Result};

fn ensure_path_exists_and_is_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(anyhow!("Path does not exist: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(anyhow!("Path is not a directory: {}", path.display()));
    }
    Ok(())
}

fn ensure_path_exists_and_is_empty_dir(path: &Path) -> Result<()> {
    ensure_path_exists_and_is_dir(path)?;
    if path.read_dir().unwrap().count() > 0 {
        return Err(anyhow!("Path is not empty: {}", path.display()));
    }
    Ok(())
}

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
}

// // TODO: optimize this
// /// this is an EXCEEDINGLY stupid way to do this and could be optimized. especially silly if the files are very differing in size.
// fn do_car_sort(in_files: Vec<PathBuf>, max_chunk_size: u64) -> Vec<(Vec<(PathBuf, u64)>, u64)> {
//     // get a list of (file, size) tuples
//     let file_sizes: Vec<(PathBuf, u64)> = in_files
//         .iter()
//         .map(|f| (f.clone(), f.metadata().unwrap().len()))
//         .collect();
//
//     // group by what's gonna go into each sector
//     let mut groups: Vec<(Vec<(PathBuf, u64)>, u64)> = Vec::new();
//     let mut current_group: Vec<(PathBuf, u64)> = Vec::new();
//     let mut current_group_size: u64 = 0;
//     for (file, size) in file_sizes {
//         if current_group_size + size > max_chunk_size {
//             groups.push((current_group, current_group_size));
//             current_group = Vec::new();
//             current_group_size = 0;
//         }
//         current_group.push((file, size));
//         current_group_size += size;
//     }
//     groups.push((current_group, current_group_size));
//
//     groups
// }

// fn make_unixfs(input_paths: Vec<PathBuf>, out_directory: PathBuf) -> Result<(), String> {
//     for car in cars {
//         let header = CarHeader::new_v1();
//         for file in car {
//
//         }
//     }
// }

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// input files as a glob
    #[arg(short, long, help = "input directories and files")]
    input: Vec<PathBuf>,

    /// output directory- must either not exist, or be an empty directory
    #[arg(short, long, help = "output directory")]
    output_dir: PathBuf,

    /// key directory - must either not exist, or be an empty directory
    #[arg(short, long, help = "key directory")]
    keys_dir: PathBuf,

    /// target size for each chunk
    #[arg(
        short,
        long,
        help = "target chunk size",
        default_value = "32_000_000_000"
    )]
    target_chunk_size: u64,

    /// should we follow symlinks?
    #[arg(short, long, help = "follow symlinks")]
    follow_symlinks: bool,
}
#[tokio::main]
async fn main() {
    let args = Args::parse();

    // get output directory
    ensure_path_exists_and_is_empty_dir(&args.output_dir)
        .expect("output directory must exist and be empty");

    // get keys directory
    ensure_path_exists_and_is_empty_dir(&args.keys_dir)
        .expect("keys directory must exist and be empty");

    // copy all the files over to an encrypted scratch directory
    let scratch_dir = args.output_dir.join("scratch");
    std::fs::create_dir(&scratch_dir).expect("could not create scratch directory");
    // copy from inputs to scratch dir
    let args_input_clone = args.input.clone();
    let _copystream =
        copy_to_scratch_space(args_input_clone, scratch_dir, args.follow_symlinks).await;

    //do_car_sort(in_files, args.target_chunk_size);

    let chunker = ChunkerConfig::from_str(&format!("fixed-{}", args.target_chunk_size)).unwrap();
    let _config = Config {
        wrap: true,
        chunker: Some(chunker),
    };

    for input_path in args.input {
        if input_path.is_symlink() && args.follow_symlinks {}
    }
}
