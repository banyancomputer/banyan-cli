use clap::Subcommand;

/// Metadata Request
#[derive(Clone, Debug, Subcommand)]
pub enum MetadataRequest {
    /// Create Metadata
    Create,
    /// Get Metadata
    Get,
    /// Delete Metadata
    Delete,
}
