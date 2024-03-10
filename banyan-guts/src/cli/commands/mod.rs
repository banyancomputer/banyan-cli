mod account;
mod api;
mod drives;
mod keys;
mod metadata;
mod runnable_command;

use std::io::Read;

use crate::native::NativeError;
pub use account::AccountCommand;
pub use api::ApiCommand;
use async_trait::async_trait;
use clap::Subcommand;
pub use drives::DrivesCommand;
pub use keys::KeyCommand;
pub use metadata::MetadataCommand;
pub use runnable_command::RunnableCommand;
pub use serde::{Deserialize, Serialize};

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone, Serialize, Deserialize)]
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
    /// Drive management
    Drives {
        /// Subcommand
        #[clap(subcommand)]
        command: DrivesCommand,
    },
}

#[async_trait]
impl RunnableCommand<NativeError> for TombCommand {
    async fn run_internal(self) -> Result<String, NativeError> {
        match self {
            TombCommand::Api { command } => Ok(command.run_internal().await?),
            TombCommand::Account { command } => Ok(command.run_internal().await?),
            TombCommand::Drives { command } => command.run_internal().await,
        }
    }
}