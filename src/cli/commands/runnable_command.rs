use std::fmt::Display;

use crate::{
    native::configuration::{globalconfig::GlobalConfig, xdg::config_path},
    WnfsError,
};
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
    async fn run_internal(self) -> Result<String, ErrorType>;

    /// Run the internal command, passing a reference to a global configuration which is saved after completion
    async fn run(self) -> Result<(), ErrorType> {
        if !config_path().exists() {
            GlobalConfig::new().await.expect("new config");
        }

        let result = self.run_internal().await;

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
