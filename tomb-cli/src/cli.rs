use clap::{Parser, Subcommand};
use log::LevelFilter;
use std::path::PathBuf;
use tomb_common as _;

// TODO add support for https://docs.rs/keyring/latest/keyring/
// TODO what's going on with buckets? these are URLs right?

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum ConfigSubCommands {
    /// new content scratch path
    ContentScratchPath {
        #[arg(
            short,
            long,
            help = "content scratch path- should be a disk of decent size where we can use it as a scratch space to build car files en route to filecoin"
        )]
        path: PathBuf,
    },
    /// tomb seturl - Set the ID for this tomb's bucket - MAY BREAK YOUR EVERYTHING!!!
    SetRemote {
        /// Input directory
        #[arg(short, long, help = "directory")]
        dir: Option<PathBuf>,
        /// Server address
        #[arg(short, long, help = "remote IPv4 address")]
        url: String,
        /// Server port
        #[arg(short, long, help = "remote address port")]
        port: u16,
    },
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
        output_dir: Option<PathBuf>,

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
    Add {
        #[arg(short, long, help = "new file / directory")]
        input_file: PathBuf,
        #[arg(short, long, help = "new file / directory")]
        tomb_path: PathBuf,
        #[arg(short, long, help = "wnfs path")]
        wnfs_path: PathBuf,
    },
    Remove {
        #[arg(short, long, help = "new file / directory")]
        tomb_path: PathBuf,
        #[arg(short, long, help = "wnfs path")]
        wnfs_path: PathBuf,
    },
    /// tomb pull - Update local from the bucket- determined by CWD
    Pull {
        /// Input directory in which packed files are stored.
        #[arg(short, long, help = "directory")]
        dir: PathBuf,
    },
    /// tomb push <bucket_name>- Push changes to a bucket to Tombolo/filecoin
    Push {
        /// Input directory in which packed files are stored.
        #[arg(short, long, help = "directory")]
        dir: PathBuf,
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
    Init {
        /// Input directory
        #[arg(short, long, help = "directory")]
        dir: Option<PathBuf>,
    },
    /// log in to tombolo remote, basically validates that your API keys or whatever are in place. must be run before registry or anything else.
    Login,
    /// tomb register <bucket_name> - Register a new bucket on the tombolo service for this data. then you can push to it. MUST be called before push.
    Register {
        /// Name of the bucket to create
        #[arg(short, long, help = "bucket name")]
        bucket_name: String,
    },
    /// tomb config <subcommand> - Configure Tombolo
    Configure {
        #[clap(subcommand)]
        subcommand: ConfigSubCommands,
    },
    Daemon,
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
