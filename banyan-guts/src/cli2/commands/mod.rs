mod account;
mod runnable_command;
mod sync;

pub use account::AccountCommand;
pub use runnable_command::RunnableCommand;
pub use sync::SyncCommand;

use crate::native::NativeError;
use clap::Subcommand;
use serde::{Deserialize, Serialize};

#[derive(Debug, Subcommand, Clone, Serialize, Deserialize)]
pub enum BanyanServiceApiCommand {
    Account {
        /// Subcommand
        #[clap(subcommand)]
        command: AccountCommand,
    },
    /// Sync management
    Sync {
        /// Subcommand
        #[clap(subcommand)]
        command: SyncCommand,
    },
}

#[async_trait::async_trait]
impl RunnableCommand<NativeError> for BanyanServiceApiCommand {
    async fn run_internal(self) -> Result<String, NativeError> {
        match self {
            BanyanServiceApiCommand::Account { command } => command.run_internal().await,
            BanyanServiceApiCommand::Sync { command } => command.run_internal().await,
        }
    }
}
