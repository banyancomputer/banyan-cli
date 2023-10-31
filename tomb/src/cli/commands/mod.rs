mod account;
mod api;
mod buckets;
mod keys;
mod metadata;
mod runnable_command;

use std::io::Read;

pub use account::*;
pub use api::*;
pub use buckets::*;
pub use keys::*;
pub use metadata::*;
pub use runnable_command::RunnableCommand;

use crate::{pipelines::error::TombError, types::config::globalconfig::GlobalConfig};
use async_trait::async_trait;
use clap::Subcommand;
use tomb_common::banyan_api::client::Client;

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
    /// Bucket management
    Buckets {
        /// Subcommand
        #[clap(subcommand)]
        command: BucketsCommand,
    },
}

#[async_trait(?Send)]
impl RunnableCommand<TombError> for TombCommand {
    async fn run_internal(
        self,
        global: &mut GlobalConfig,
        client: &mut Client,
    ) -> Result<String, TombError> {
        match self {
            TombCommand::Api { command } => Ok(command.run_internal(global, client).await?),
            TombCommand::Account { command } => Ok(command.run_internal(global, client).await?),
            TombCommand::Buckets { command } => command.run_internal(global, client).await,
        }
    }
}
