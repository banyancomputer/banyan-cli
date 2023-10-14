use crate::{
    cli::commands::{AddressCommand, ApiCommand},
    types::config::globalconfig::GlobalConfig,
};
use anyhow::{anyhow, Result};
use reqwest::Url;

/// Verify the integrity of a provided address
fn verify_address(address: &str) -> Result<()> {
    // Update if the address is valid
    if Url::parse(address).is_ok() {
        Ok(())
    } else {
        Err(anyhow!("<< ADDRESS WAS NOT FORMATTED CORRECTLY >>"))
    }
}

/// Process an individual global configuration field
fn process_field(
    label: &str,
    field: &mut String,
    address: Option<AddressCommand>,
) -> Result<String> {
    match address {
        None => Ok(format!("<< CURRENT {} ADDRESS >>\n{}\n", label, field)),
        Some(AddressCommand::Set { address }) => {
            // Verify the address
            verify_address(&address)?;
            // Update the address
            *field = address;
            // Report okay
            Ok("<< CONFIGURATION UPDATED SUCCESSFULLY >>".to_string())
        }
    }
}

/// Handle Bucket management both locally and remotely based on CLI input
pub async fn pipeline(command: ApiCommand) -> Result<String> {
    let mut global = GlobalConfig::from_disk().await?;
    let result = match command {
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
    };
    // Save the config
    global.to_disk()?;
    // Ok
    result
}
