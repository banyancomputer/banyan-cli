use super::DriveSpecifier;
use clap::Args;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unified way of specifying a Key
#[derive(Debug, Clone, Args, Deserialize, Serialize)]
pub struct MetadataSpecifier {
    #[clap(flatten)]
    pub(crate) drive_specifier: DriveSpecifier,
    /// Uuid of the Metadata
    #[arg(short, long)]
    pub(crate) metadata_id: Uuid,
}
