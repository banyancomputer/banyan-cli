use std::path::PathBuf;

use clap::{arg, Subcommand};
use tomb_common::api::request::{BucketRequest, CreateBucketRequest, Request};

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    /// Packing a filesystem on disk into an encrypted WNFS CAR file
    Pack {
        /// Root of the directory tree to pack.
        #[arg(short, long, help = "input directories and files")]
        origin: Option<PathBuf>,

        // /// Maximum size for each chunk, defaults to 1GiB.
        // #[arg(short, long, help = "target chunk size", default_value = "1073741824")]
        // chunk_size: u64,
        /// Whether to follow symbolic links when processing the input directory.
        #[arg(short, long, help = "follow symbolic links")]
        follow_links: bool,
        // TODO add support for GroupConfig::path_patterns/name_patterns
    },
    /// Reconstructing a filesystem from an encrypted WNFS CAR file
    Unpack {
        /// Origin path
        #[arg(short, long, help = "path to original filesystem")]
        origin: Option<PathBuf>,

        /// Output directory in which reinflated files will be unpacked.
        #[arg(short, long, help = "output directory for filesystem reconstruction")]
        unpacked: PathBuf,
    },
    /// Add an individual file or folder to an existing bucket
    Add {
        /// Origin path
        #[arg(short, long, help = "original input directory")]
        origin: PathBuf,

        /// Path of file / folder being added
        #[arg(short, long, help = "new file / directory")]
        input_file: PathBuf,

        /// Path at which the node will be added in the WNFS
        #[arg(short, long, help = "wnfs path")]
        wnfs_path: PathBuf,
    },
    /// Remove an individual file or folder from an existing bucket
    Remove {
        /// Origin path
        #[arg(short, long, help = "original input directory")]
        origin: PathBuf,

        /// Path at which the node will be removed from the WNFS if it exists
        #[arg(short, long, help = "wnfs path")]
        wnfs_path: PathBuf,
    },
    /// Create new bucket config for a directory
    Init {
        /// Directory to init, or PWD if None
        dir: Option<PathBuf>,
    },
    /// Remove config and packed data for a directory
    Deinit {
        /// Directory to deinit, or PWD if None
        dir: Option<PathBuf>,
    },
    /// log in to tombolo remote, basically validates that your API keys or whatever are in place. must be run before registry or anything else.
    Login,
    /// tomb config <subcommand> - Configure Tombolo
    Configure {
        /// Configuration subcommand
        #[clap(subcommand)]
        subcommand: ConfigSubCommand,
    },
    /// We don't know yet
    Daemon,
    /// Interact with Banyan Metadata API
    Api {
        /// Request subcommand
        #[clap(subcommand)]
        subcommand: Request,
    },
}

/// Sub-commands associated with configuration
#[derive(Subcommand, Clone, Debug)]
pub enum ConfigSubCommand {
    /// Set the remote endpoint where buckets are synced to / from
    SetRemote {
        /// Server address
        #[arg(short, long, help = "full server address")]
        address: String,
    },
}
