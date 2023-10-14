mod account;
mod api;
mod buckets;
mod keys;
mod metadata;

pub use account::*;
pub use api::*;
pub use buckets::*;
pub use keys::*;
pub use metadata::*;

use crate::types::config::globalconfig::GlobalConfig;
use async_trait::async_trait;
use clap::Subcommand;
use tomb_common::banyan_api::client::Client;

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

/// Async function for running a command
#[async_trait(?Send)]
pub trait RunnableCommand<ErrorType: std::error::Error>: Subcommand {
    /// The internal running operation
    async fn run_internal(
        &self,
        global: &mut GlobalConfig,
        client: &mut Client,
    ) -> Result<String, ErrorType>;

    /// Run the internal command, passing a reference to a global configuration which is saved after completion
    async fn run(&self) -> Result<String, ErrorType> {
        // Grab global config
        let mut global = GlobalConfig::from_disk()
            .await
            .expect("unable to load global config");
        let mut client = global.get_client().await.expect("unable to load client");
        let result = self.run_internal(&mut global, &mut client).await;
        global
            .save_client(client)
            .await
            .expect("unable to save client to config");
        global.to_disk().expect("Unable to save global config");
        result
    }
}
