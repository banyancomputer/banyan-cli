use super::BucketSpecifier;
use clap::Args;
use uuid::Uuid;

/// Unified way of specifying a Key
#[derive(Debug, Clone, Args)]
pub struct MetadataSpecifier {
    #[clap(flatten)]
    pub(crate) bucket_specifier: BucketSpecifier,
    /// Uuid of the Metadata
    #[arg(short, long)]
    pub(crate) metadata_id: Uuid,
}
