use clap::{Parser, Subcommand};
use log::LevelFilter;
use std::path::PathBuf;

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub(crate) enum Commands {
    Pack {
        /// Root of the directory tree to pack.
        #[arg(short, long, help = "input directories and files")]
        input_dir: PathBuf,

        /// Directory that either does not exist or is empty; this is where packed data will go.
        #[arg(short, long, help = "output directory")]
        output_dir: PathBuf,

        /// Maximum size for each chunk, defaults to 1GiB.
        #[arg(short, long, help = "target chunk size", default_value = "1073741824")]
        chunk_size: u64,

        /// Whether to follow symbolic links when processing the input directory.
        #[arg(short, long, help = "follow symbolic links")]
        follow_links: bool,
        // TODO add support for GroupConfig::path_patterns/name_patterns
    },
    Unpack {
        /// Input directory in which packed files are stored.
        #[arg(short, long, help = "input directory")]
        input_dir: PathBuf,

        /// Output directory in which reinflated files will be unpacked.
        #[arg(short, long, help = "output directory")]
        output_dir: PathBuf,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub(crate) enum MyVerbosity {
    Quiet,
    Normal,
    Verbose,
    VeryVerbose,
    Debug,
}

impl From<MyVerbosity> for LevelFilter {
    fn from(val: MyVerbosity) -> Self {
        match val {
            MyVerbosity::Quiet => LevelFilter::Off,
            MyVerbosity::Normal => LevelFilter::Info,
            MyVerbosity::Verbose => LevelFilter::Debug,
            MyVerbosity::VeryVerbose => LevelFilter::Trace,
            MyVerbosity::Debug => LevelFilter::Trace,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub(crate) command: Commands,

    /// Verbosity level.
    #[arg(short, long, help = "verbosity level", default_value = "normal")]
    pub(crate) verbose: MyVerbosity,
}
