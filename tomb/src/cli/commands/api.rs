use super::RunnableCommand;
use crate::types::config::globalconfig::GlobalConfig;
use async_trait::async_trait;
use clap::Subcommand;
use reqwest::Url;
use tomb_common::banyan_api::client::Client;

/// Subcommand for endpoint configuration
#[derive(Subcommand, Clone, Debug)]
pub enum ApiCommand {
    /// Address of Core server
    Core {
        /// Server address
        #[clap(subcommand)]
        command: Option<AddressCommand>,
    },
    /// Address of Data server
    Data {
        /// Server address
        #[clap(subcommand)]
        command: Option<AddressCommand>,
    },
    /// Address of Frontend server
    Frontend {
        /// Server address
        #[clap(subcommand)]
        command: Option<AddressCommand>,
    },
}

/// Subcommand for getting and setting remote addresses
#[derive(Subcommand, Clone, Debug)]
pub enum AddressCommand {
    /// Set the address to a new value
    Set {
        /// Server address
        #[arg(short, long)]
        address: String,
    },
}

#[async_trait(?Send)]
impl RunnableCommand<anyhow::Error> for ApiCommand {
    async fn run_internal(
        self,
        global: &mut GlobalConfig,
        _: &mut Client,
    ) -> Result<String, anyhow::Error> {
        match self {
            // Core service
            ApiCommand::Core { command: address } => {
                process_field("CORE", &mut global.remote_core, address)
            }
            // Data service
            ApiCommand::Data { command: address } => {
                process_field("DATA", &mut global.remote_data, address)
            }
            // Frontend service
            ApiCommand::Frontend { command: address } => {
                process_field("FRONTEND", &mut global.remote_frontend, address)
            }
        }
    }
}

/// Process an individual global configuration field
fn process_field(
    label: &str,
    field: &mut String,
    address: Option<AddressCommand>,
) -> anyhow::Result<String> {
    match address {
        None => Ok(format!("<< CURRENT {} ADDRESS >>\n{}\n", label, field)),
        Some(AddressCommand::Set { address }) => {
            // Verify the address
            verify_address(&address)?;
            // Update the address
            *field = address.to_string();
            // Report okay
            Ok("<< CONFIGURATION UPDATED SUCCESSFULLY >>".to_string())
        }
    }
}

/// Verify the integrity of a provided address
fn verify_address(address: &str) -> anyhow::Result<()> {
    // Update if the address is valid
    if Url::parse(address).is_ok() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("<< ADDRESS WAS NOT FORMATTED CORRECTLY >>"))
    }
}
