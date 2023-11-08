use super::DriveSpecifier;
use clap::Args;

/// Unified way of specifying a Key
#[derive(Debug, Clone, Args)]
pub struct KeySpecifier {
    #[clap(flatten)]
    pub(crate) drive_specifier: DriveSpecifier,
    /// Key Identifier
    #[arg(short, long)]
    pub(crate) fingerprint: String,
}
