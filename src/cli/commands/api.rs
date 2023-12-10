use super::RunnableCommand;
use crate::{
    api::client::Client,
    native::{configuration::globalconfig::GlobalConfig, NativeError},
};
use async_trait::async_trait;
use clap::Subcommand;
use colored::Colorize;
use reqwest::Url;

/// Subcommand for endpoint configuration
#[derive(Subcommand, Clone, Debug)]
pub enum ApiCommand {
    /// Display the current remote endpoint
    Display,
    /// Set the endpoint to a new value
    Set {
        /// Server address
        #[arg(short, long)]
        address: String,
    },
    /// Return the endpoint to the original values
    Reset,
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for ApiCommand {
    async fn run_internal(
        self,
        mut global: GlobalConfig,
        _: Client,
    ) -> Result<String, NativeError> {
        match self {
            ApiCommand::Display => Ok(format!(
                "{}\n{}\n",
                "| ADDRESS INFO |".yellow(),
                global.endpoint
            )),
            ApiCommand::Set { address } => {
                global.endpoint = Url::parse(&address).map_err(|_| NativeError::bad_data())?;
                global.to_disk()?;
                Ok(format!("{}", "<< ENDPOINT UPDATED SUCCESSFULLY >>".green()))
            }
            ApiCommand::Reset => {
                let endpoint = Url::parse(if option_env!("DEV_ENDPOINTS").is_some() {
                    "http://127.0.0.1:3001"
                } else {
                    "https://alpha.data.banyan.computer"
                })
                .expect("unable to parse known URLs");
                global.endpoint = endpoint;
                global.to_disk()?;
                Ok(format!("{}", "<< ENDPOINTS HAVE BEEN RESET >>".green()))
            }
        }
    }
}
