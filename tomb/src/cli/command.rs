use std::path::{Path, PathBuf};

use clap::{arg, Args, Subcommand};
use uuid::Uuid;

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    /// Set the remote endpoint where Buckets are synced to / from
    SetRemoteCore {
        /// Server address
        #[arg(short, long)]
        address: String,
    },
    /// Set the remote endpoint where Buckets are synced to / from
    SetRemoteData {
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
// #[group(required = true, multiple = false)]
pub struct BucketSpecifier {
    /// Bucket Id
    #[arg(short, long)]
    pub bucket_id: Option<Uuid>,
    /// Bucket Root
    #[arg(short, long)]
    pub origin: Option<PathBuf>,
}

impl BucketSpecifier {
    /// Create a new BucketSpecifier with an Id
    pub fn with_id(id: Uuid) -> Self {
        Self {
            bucket_id: Some(id),
            origin: None,
        }
    }

    /// Create a new BucketSpecifier with a Path
    pub fn with_origin(path: &Path) -> Self {
        Self {
            bucket_id: None,
            origin: Some(path.to_path_buf()),
        }
    }
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
    /// Encrypt / Bundle a Bucket
    Bundle {
        /// Bucket in question
        #[clap(flatten)]
        bucket_specifier: BucketSpecifier,

        /// Follow symbolic links
        #[arg(short, long)]
        follow_links: bool,
    },
    /// Decrypt / Extract a Bucket
    Extract {
        /// Bucket in question
        #[clap(flatten)]
        bucket_specifier: BucketSpecifier,

        /// Output Directory
        #[arg(short, long)]
        output: PathBuf,
    },
    /// List all Buckets
    List,
    /// Delete Bucket
    Delete(BucketSpecifier),
    /// Bucket info
    Info(BucketSpecifier),
    /// Bucket usage
    Usage(BucketSpecifier),
    /// Metadata uploads and downloads
    Metadata {
        /// Subcommand
        #[clap(subcommand)]
        subcommand: MetadataSubCommand,
    },
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
    pub(crate) bucket_specifier: BucketSpecifier,
    /// Key Identifier
    #[arg(short, long)]
    pub(crate) fingerprint: String,
}

/// Subcommand for Bucket Keys
#[derive(Subcommand, Clone, Debug)]
pub enum KeySubCommand {
    /// List all Keys in a Bucket
    List(BucketSpecifier),
    /// Request Access to a Bucket if you dont already have it
    RequestAccess(BucketSpecifier),
    /// Delete a given Key
    Delete(KeySpecifier),
    /// List the keys persisted by the remote endpoint
    Info(KeySpecifier),
    /// Reject or remove a key and sync that witht the remote endpoint
    Reject(KeySpecifier),
}

/// Subcommand for Bucket Metadata
#[derive(Subcommand, Clone, Debug)]
pub enum MetadataSubCommand {
    /// Read an individual Metadata Id
    Read {
        /// Bucket in question
        #[clap(flatten)]
        bucket_specifier: BucketSpecifier,

        /// Id of the Metadata
        #[arg(short, long)]
        metadata_id: Uuid,
    },
    /// Read the currently active Metadata
    ReadCurrent(BucketSpecifier),
    /// List all Metadatas associated with Bucket
    List(BucketSpecifier),
    /// Upload Metadata
    Push(BucketSpecifier),
    /// Download Metadata
    Pull {
        /// Bucket in question
        #[clap(flatten)]
        bucket_specifier: BucketSpecifier,

        /// Id of the Metadata
        #[arg(short, long)]
        metadata_id: Uuid,
    },
    /// Grab Snapshot
    Snapshot {
        /// Bucket in question
        #[clap(flatten)]
        bucket_specifier: BucketSpecifier,

        /// Id of the Metadata
        #[arg(short, long)]
        metadata_id: Uuid,
    },
}
