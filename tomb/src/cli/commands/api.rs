use super::RunnableCommand;
use crate::types::config::{globalconfig::GlobalConfig, Endpoints};
use async_trait::async_trait;
use clap::Subcommand;
use colored::Colorize;
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
    /// Return these addresses to their original values
    Reset,
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
                process_field("CORE", &mut global.endpoints.core, address)
            }
            // Data service
            ApiCommand::Data { command: address } => {
                process_field("DATA", &mut global.endpoints.data, address)
            }
            // Frontend service
            ApiCommand::Frontend { command: address } => {
                process_field("FRONTEND", &mut global.endpoints.frontend, address)
            }
            ApiCommand::Reset => {
                global.endpoints = Endpoints::default();
                Ok(format!("{}", "<< ENDPOINTS HAVE BEEN RESET >>".green()))
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
        None => Ok(format!(
            "{}\n{}\n",
            format!("| {} ADDRESS INFO |", label).yellow(),
            field
        )),
        Some(AddressCommand::Set { address }) => {
            // Verify the address
            verify_address(&address)?;
            // Update the address
            *field = address;
            // Report okay
            Ok(format!(
                "{}",
                format!("<< {} ENDPOINT UPDATED SUCCESSFULLY >>", label).green()
            ))
        }
    }
}

/// Verify the integrity of a provided address
fn verify_address(address: &str) -> anyhow::Result<()> {
    // Update if the address is valid
    Url::parse(address)
        .map(|_| ())
        .map_err(|_| anyhow::anyhow!("ADDRESS WAS NOT FORMATTED CORRECTLY"))
}
