use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Subcommand, Clone)]
pub enum Commands {
    Pack {
        /// input file root to spider
        #[arg(short, long, help = "input directories and files")]
        input_dir: PathBuf,

        /// output directory- must either not exist, or be an empty directory
        #[arg(short, long, help = "output directory")]
        output_dir: PathBuf,

        /// where to put the manifest file
        #[arg(short, long, help = "manifest file location")]
        manifest_file: PathBuf,

        /// target size for each chunk (default is one gig)
        #[arg(short, long, help = "target chunk size", default_value = "1073741824")]
        target_chunk_size: u64,

        /// whether to follow symbolic links
        #[arg(short, long, help = "follow symbolic links")]
        follow_links: bool,
        // todo add support for GroupConfig::path_patterns/name_patterns
    },
    Unpack {
        /// input file root
        #[arg(short, long, help = "input directories and files")]
        input_dir: PathBuf,

        /// where to get the manifest file
        #[arg(short, long, help = "manifest file location")]
        manifest_file: PathBuf,

        /// output directory to repopulate with reinflated files
        #[arg(short, long, help = "output directory")]
        output_dir: PathBuf,
    },
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}
