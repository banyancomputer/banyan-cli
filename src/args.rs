use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// input file root to spider
    #[arg(short, long, help = "input directories and files")]
    pub input_dir: PathBuf,

    /// output directory- must either not exist, or be an empty directory
    #[arg(short, long, help = "output directory")]
    pub output_dir: PathBuf,

    /// where to put the manifest file
    #[arg(short, long, help = "manifest file location")]
    pub manifest_file: PathBuf,

    /// target size for each chunk (default is one gig)
    #[arg(short, long, help = "target chunk size", default_value = "1073741824")]
    pub target_chunk_size: u64,

    /// should we follow symlinks?
    /// fed into: https://docs.rs/jwalk/latest/jwalk/struct.WalkDirGeneric.html#method.follow_links
    #[arg(short, long, help = "follow symlinks", default_value = "false")]
    pub follow_links: bool,
}
