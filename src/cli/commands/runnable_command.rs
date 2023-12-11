use std::fmt::Display;

use crate::{api::client::Client, native::configuration::globalconfig::GlobalConfig, WnfsError};
use async_trait::async_trait;
use clap::Subcommand;
use colored::Colorize;

/// Async function for running a command
#[async_trait(?Send)]
pub trait RunnableCommand<ErrorType>: Subcommand
where
    ErrorType: Into<WnfsError> + std::fmt::Debug + Display,
{
    /// The internal running operation
    async fn run_internal(
        self,
        mut global: GlobalConfig,
        mut client: Client,
    ) -> Result<String, ErrorType>;

    /// Run the internal command, passing a reference to a global configuration which is saved after completion
    async fn run(self) -> Result<(), ErrorType> {
        // Grab global config
        let global = GlobalConfig::from_disk().await.unwrap_or(
            GlobalConfig::new()
                .await
                .expect("unable to create new config"),
        );
        let client = global.get_client().await.expect("unable to load client");
        let result = self.run_internal(global, client).await;

        // Provide output based on that
        match result {
            Ok(message) => {
                info!("{}", message);
                Ok(())
            }
            Err(error) => {
                error!("{}", format!("{}", error).red());
                Err(error)
            }
        }
    }
}
