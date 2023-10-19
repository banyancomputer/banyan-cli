use std::path::{Path, PathBuf};

use clap::{arg, Args, Subcommand};
use uuid::Uuid;

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub enum Command {
    /// Set API endpoints
    Configure {
        /// Subcommand
        #[clap(subcommand)]
        subcommand: ConfigureSubCommand,
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

/// Subcommand for getting and setting remote addresses
#[derive(Subcommand, Clone, Debug)]
pub enum AddressSubCommand {
    /// Print the address as it is currently configured
    Get,
    /// Set the address to a new value
    Set {
        /// Server address
        #[arg(short, long)]
        address: String,
    },
}

/// Subcommand for endpoint configuration
#[derive(Subcommand, Clone, Debug)]
pub enum ConfigureSubCommand {
    /// Address of Core server
    Core {
        /// Server address
        #[clap(subcommand)]
        address: AddressSubCommand,
    },
    /// Address of Data server
    Data {
        /// Server address
        #[clap(subcommand)]
        address: AddressSubCommand,
    },
    /// Address of Frontend server
    Frontend {
        /// Server address
        #[clap(subcommand)]
        address: AddressSubCommand,
    },
}

/// Subcommand for Authentication
#[derive(Subcommand, Clone, Debug)]
pub enum AuthSubCommand {
    /// Add Device API Key
    RegisterDevice,
    /// Register
    #[cfg(feature = "fake")]
    Register,
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

/// Unified way of specifying a Key
#[derive(Debug, Clone, Args)]
pub struct MetadataSpecifier {
    #[clap(flatten)]
    pub(crate) bucket_specifier: BucketSpecifier,
    /// Uuid of the Metadata
    #[arg(short, long)]
    pub(crate) metadata_id: Uuid,
}

/// Subcommand for Bucket Metadata
#[derive(Subcommand, Clone, Debug)]
pub enum MetadataSubCommand {
    /// Read an individual Metadata Id
    Read(MetadataSpecifier),
    /// Read the currently active Metadata
    ReadCurrent(BucketSpecifier),
    /// List all Metadatas associated with Bucket
    List(BucketSpecifier),
    /// Upload Metadata
    Push(BucketSpecifier),
    /// Download Metadata
    Pull(MetadataSpecifier),
    /// Grab Snapshot
    Snapshot(MetadataSpecifier),
}
