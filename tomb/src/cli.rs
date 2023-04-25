use clap::{Parser, Subcommand};
use log::LevelFilter;
use std::path::PathBuf;

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum PathConfigSubCommands {
    Metadata{
        /// new metadata path
        #[arg(short, long, help = "new metadata path")]
        path: PathBuf,
    },
    Content{
        /// new content path
        #[arg(short, long, help = "new content path")]
        path: PathBuf,
    },
    Bucket{
        /// new bucket path
        #[arg(short, long, help = "new bucket path")]
        path: PathBuf,
    },
    Index{
        /// new index path
        #[arg(short, long, help = "new index path")]
        path: PathBuf,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum TomboloConfigSubCommands {
    SetKey{
        /// new tombolo api key
        #[arg(short, long, help = "new key")]
        key: String,
    },
}

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum ConfigSubCommands {
    #[command(subcommand)]
    Path(PathConfigSubCommands),
    #[command(subcommand)]
    Tombolo(TomboloConfigSubCommands),
}

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
    ///- Initialize Tomb - Abort if the `~/.tomb` path already exists
    //- Create a new directory at `~/.tomb`
    //- Create a new config file at `~/.tomb/config`:
    //    - `metadata_path: ~/.tomb/metadata`
    //    - `content_path: ~/.tomb/content`
    //    - `bucket_path: ~./tomb/buckets`
    //    - `tombolo_path: ~/.tomb/olo`
    //    - `index_path: ~/.tomb/index`
    /// tomb init - create a new .tomb file and populate it.
    Init,
    /// tomb config <subcommand> - Configure Tombolo
    Configure {
        #[clap(subcommand)]
        subcommand: ConfigSubCommands,
    },
    /// tomb new <bucket_name> -p <path_to_master> - Create a new bucket
    New {
        /// Name of the bucket to create
        #[arg(short, long, help = "bucket name")]
        bucket_name: String,

        /// Path to master
        #[arg(short, long, help = "path to master")]
        master_path: PathBuf,
    },
    /// tomb update <bucket_name> - Update a bucket from master. 
    Update {
        /// Name of the bucket to update
        #[arg(short, long, help = "bucket name")]
        bucket_name: String,
    },
    /// tomb push <bucket_name>- Push changes to a bucket to Tombolo
    Push {
        /// Name of the bucket to push
        #[arg(short, long, help = "bucket name")]
        bucket_name: String,
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
