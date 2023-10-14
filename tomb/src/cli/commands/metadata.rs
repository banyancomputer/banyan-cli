use super::super::specifiers::*;
use clap::Subcommand;

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
