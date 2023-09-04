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
    /// Encrypt / Bundle a Bucket
    Bundle(BucketSpecifier),
    /// Decrypt / Extract a Bucket
    Extract {
        /// Bucket in question
        #[clap(flatten)]
        bucket_specifier: BucketSpecifier,

        /// Output Directory
        output: PathBuf,
    },
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
