use std::path::PathBuf;

use clap::{arg, Subcommand};
use uuid::Uuid;

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    /// Set the remote endpoint where Buckets are synced to / from
    SetRemote {
        /// Server address
        #[arg(short, long, help = "full server address")]
        address: String,
    },
    /// Login, Register, etc.
    Auth {
        #[clap(subcommand)]
        subcommand: AuthSubCommand,
    },
    /// Bucket management
    Bucket {
        #[clap(subcommand)]
        subcommand: BucketSubCommand,
    },

    /// Packing a filesystem on disk into an encrypted WNFS CAR file
    Pack {
        /// Bucket Root
        #[arg(short, long, help = "bucket dir")]
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

#[derive(Subcommand, Clone, Debug)]
pub enum BucketSubCommand {
    /// Initialize a new Bucket locally
    Create {
        /// Bucket Root
        #[arg(short, long, help = "bucket root")]
        origin: Option<PathBuf>,

        /// Bucket Name
        #[arg(short, long, help = "bucket name")]
        name: String,
    },
    /// List all Buckets
    List,
    /// Modify an existing Bucket
    Modify {
        /// Bucket Root
        #[arg(short, long, help = "bucket root")]
        origin: Option<PathBuf>,

        #[clap(subcommand)]
        subcommand: ModifyBucketSubCommand
    }
}


#[derive(Subcommand, Clone, Debug)]
pub enum ModifyBucketSubCommand {
    // /// Sync metadata
    // Sync,
    /// Publish Bucket content
    Push,
    /// Pull
    Pull,
    /// Delete Bucket
    Delete,
    /// Bucket info
    Info,
    /// Bucket usage
    Usage,
    /// Bucket Key management
    Keys {
        #[clap(subcommand)]
        subcommand: KeySubCommand,
    }
}

#[derive(Subcommand, Clone, Debug)]
pub enum KeySubCommand {
    List,
    Create,
    Modify {
        /// Key Identifier
        #[arg(short, long, help = "key identifier")]
        id: Uuid,

        /// Subcommand
        #[clap(subcommand)]
        subcommand: ModifyKeySubCommand,
    }
}

#[derive(Subcommand, Clone, Debug)]
pub enum ModifyKeySubCommand {
    Delete,
    /// List the keys persisted by the remote endpoint
    Info,
    /// Approve a key for use and sync that with the remote endpoint
    Approve,
    /// Reject or remove a key and sync that witht the remote endpoint
    Reject,
}
