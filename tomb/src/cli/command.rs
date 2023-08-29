use std::path::PathBuf;

use clap::{arg, Args, Subcommand};
use uuid::Uuid;

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    /// Set the remote endpoint where Buckets are synced to / from
    SetRemote {
        /// Server address
        #[arg(short, long)]
        address: String,
    },
    /// Login, Register, etc.
    Auth {
        /// Subcommand
        #[clap(subcommand)]
        subcommand: AuthSubCommand,
    },
    /// Bucket management
    Buckets {
        /// Subcommand
        #[clap(subcommand)]
        subcommand: BucketsSubCommand,
    },
    /// Packing a filesystem on disk into an encrypted WNFS CAR file
    Pack {
        /// Bucket Root
        #[arg(short, long)]
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
}

/// Subcommand for Authentication
#[derive(Subcommand, Clone, Debug)]
pub enum AuthSubCommand {
    /// Create an account
    Register,
    /// Login to an existing account
    Login,
    /// Ask the server who I am
    WhoAmI,
    /// Ask the server my usage
    Usage,
    /// Ask the server my usage limit
    Limit,
}


/// Unified way of specifying a Bucket
#[derive(Debug, Clone, Args)]
#[group(required = true, multiple = false)]
pub struct BucketSpecifier {
    /// Bucket Id
    #[arg(short, long)]
    pub bucket_id: Option<Uuid>,
    /// Bucket Root
    #[arg(short, long)]
    pub origin: Option<PathBuf>,
}

/// Subcommand for Bucket Management
#[derive(Subcommand, Clone, Debug)]
pub enum BucketsSubCommand {
    /// Initialize a new Bucket locally
    Create {
        /// Bucket Name
        #[arg(short, long)]
        name: String,
        /// Bucket Root
        #[arg(short, long)]
        origin: Option<PathBuf>,
    },
    /// List all Buckets
    List,
    /// Publish Bucket content
    Push(BucketSpecifier),
    /// Pull
    Pull(BucketSpecifier),
    /// Delete Bucket
    Delete(BucketSpecifier),
    /// Bucket info
    Info(BucketSpecifier),
    /// Bucket usage
    Usage(BucketSpecifier),
    /// Bucket Key management
    Keys {
        /// Subcommand
        #[clap(subcommand)]
        subcommand: KeySubCommand,
    },
}

/// Unified way of specifying a Key
#[derive(Debug, Clone, Args)]
pub struct KeySpecifier {
    #[clap(flatten)]
    pub(crate) bucket: BucketSpecifier,
    /// Key Identifier
    #[arg(short, long)]
    pub(crate) key_id: Uuid,
}

/// Subcommand for Bucket Keys
#[derive(Subcommand, Clone, Debug)]
pub enum KeySubCommand {
    /// List all Keys in a Bucket
    List(BucketSpecifier),
    /// Create a new Key for a Bucket
    Create(BucketSpecifier),
    /// Delete a given Key
    Delete(KeySpecifier),
    /// List the keys persisted by the remote endpoint
    Info(KeySpecifier),
    /// Approve a key for use and sync that with the remote endpoint
    Approve(KeySpecifier),
    /// Reject or remove a key and sync that witht the remote endpoint
    Reject(KeySpecifier),
}
