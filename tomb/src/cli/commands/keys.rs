use super::super::specifiers::*;
use clap::Subcommand;

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
