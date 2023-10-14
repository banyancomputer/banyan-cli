mod account;
mod api;
mod buckets;
mod keys;
mod metadata;

pub use account::*;
pub use api::*;
pub use buckets::*;
use clap::Subcommand;
pub use keys::*;
pub use metadata::*;

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub enum TombCommand {
    /// Manually configure remote endpoints
    Api {
        /// Subcommand
        #[clap(subcommand)]
        command: ApiCommand,
    },
    /// Account Login and Details
    Account {
        /// Subcommand
        #[clap(subcommand)]
        command: AccountCommand,
    },
    /// Bucket management
    Buckets {
        /// Subcommand
        #[clap(subcommand)]
        command: BucketsCommand,
    },
}
