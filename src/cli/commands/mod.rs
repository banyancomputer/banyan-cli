mod account;
mod api;
mod drives;
mod keys;
mod metadata;
mod runnable_command;

use std::io::Read;

pub use account::AccountCommand;
pub use api::ApiCommand;
pub use drives::DrivesCommand;
pub use keys::KeyCommand;
pub use metadata::MetadataCommand;
pub use runnable_command::RunnableCommand;

use crate::{
    api::client::Client,
    native::{configuration::globalconfig::GlobalConfig, operations::OperationError},
};
use async_trait::async_trait;
use clap::Subcommand;

use super::CliError;

/// Prompt the user for a y/n answer
pub fn prompt_for_bool(msg: &str) -> bool {
    info!("{msg} y/n");
    loop {
        let mut input = [0];
        let _ = std::io::stdin().read(&mut input);
        match input[0] as char {
            'y' | 'Y' => return true,
            'n' | 'N' => return false,
            _ => info!("y/n only please."),
        }
    }
}

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
    /// Drive management
    Drives {
        /// Subcommand
        #[clap(subcommand)]
        command: DrivesCommand,
    },
}

#[async_trait(?Send)]
impl RunnableCommand<CliError> for TombCommand {
    async fn run_internal(
        self,
        global: &mut GlobalConfig,
        client: &mut Client,
    ) -> Result<String, CliError> {
        match self {
            TombCommand::Api { command } => Ok(command.run_internal(global, client).await?),
            TombCommand::Account { command } => Ok(command.run_internal(global, client).await?),
            TombCommand::Drives { command } => command.run_internal(global, client).await,
        }
    }
}
