use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub enum Commands {
    Pack {
        /// Root of the directory tree to pack.
        #[arg(short, long, help = "input directories and files")]
        input_dir: PathBuf,

        /// Directory that either does not exist or is empty; this is where packed data will go.
        #[arg(short, long, help = "output directory")]
        output_dir: PathBuf,

        /// Location in which the manifest file will be written.
        #[arg(short, long, help = "manifest file location")]
        manifest_file: PathBuf,

        /// Maximum size for each chunk, defaults to 1GiB.
        #[arg(short, long, help = "target chunk size", default_value = "1073741824")]
        chunk_size: u64,

        /// Whether to follow symbolic links when processing the input directory.
        #[arg(short, long, help = "follow symbolic links")]
        follow_links: bool,
        // TODO add support for GroupConfig::path_patterns/name_patterns
    },
    Unpack {
        /// Input directory in which packed files are located.
        #[arg(short, long, help = "input directory")]
        input_dir: PathBuf,

        /// Output directory in which reinflated files will be unpacked.
        #[arg(short, long, help = "output directory")]
        output_dir: PathBuf,

        /// Location of the manifest file.
        #[arg(short, long, help = "manifest file location")]
        manifest_file: PathBuf,
    },
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}
