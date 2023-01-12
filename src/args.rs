use std::path::PathBuf;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    // /// input files as a glob
    //#[arg(short, long, help = "input directories and files")]
    //pub input: Vec<PathBuf>,

    /// output directory- must either not exist, or be an empty directory
    #[arg(short, long, help = "output directory")]
    pub output_dir: PathBuf,

    // /// key directory - must either not exist, or be an empty directory
    //#[arg(short, long, help = "key directory")]
    //pub keys_dir: PathBuf,

    // /// target size for each chunk
    //#[arg(
    //short,
    //long,
    //help = "target chunk size",
    //default_value = "32000000000"
    //)]
    //pub target_chunk_size: u64,

    // /// should we follow symlinks?
    //#[arg(short, long, help = "follow symlinks")]
    //pub follow_symlinks: bool,
}

