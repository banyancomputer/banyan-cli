use super::DriveSpecifier;
use clap::Args;
use serde::{Deserialize, Serialize};

/// Unified way of specifying a Key
#[derive(Debug, Clone, Args, Serialize, Deserialize)]
pub struct KeySpecifier {
    #[clap(flatten)]
    pub(crate) drive_specifier: DriveSpecifier,
    /// Key Identifier
    #[arg(short, long)]
    pub(crate) fingerprint: String,
}
