use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// input files as a glob
    #[arg(short, long, help = "input directories and files")]
    pub input_dirs: Vec<PathBuf>,

    /// output directory- must either not exist, or be an empty directory
    #[arg(short, long, help = "output directory")]
    pub output_dir: PathBuf,

    // Note (amiller68): We don't necessarily need to create the keys dir, removing for now.
    // /// key directory - must either not exist, or be an empty directory
    // #[arg(short, long, help = "key directory")]
    // pub keys_dir: PathBuf,
    /// target size for each chunk
    #[arg(short, long, help = "target chunk size", default_value = "32000000000")]
    pub target_chunk_size: u64,

    /// should we follow symlinks?
    /// fed into: https://docs.rs/jwalk/latest/jwalk/struct.WalkDirGeneric.html#method.follow_links
    #[arg(short, long, help = "follow symlinks", default_value = "false")]
    pub follow_links: bool,
}
