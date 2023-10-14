use super::{super::specifiers::BucketSpecifier, KeySubCommand, MetadataSubCommand};
use clap::Subcommand;
use std::path::PathBuf;

/// Subcommand for Bucket Management
#[derive(Subcommand, Clone, Debug)]
pub enum BucketsCommand {
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
