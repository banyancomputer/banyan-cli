use crate::{
    cli::command::{AddressSubCommand, ConfigureSubCommand},
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
fn process_field(field: &mut String, address: AddressSubCommand) -> Result<String> {
    match address {
        AddressSubCommand::Get => Ok(field.clone()),
        AddressSubCommand::Set { address } => {
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
pub async fn pipeline(command: ConfigureSubCommand) -> Result<String> {
    let mut global = GlobalConfig::from_disk().await?;
    let result = match command {
        // Core service
        ConfigureSubCommand::Core { address } => process_field(&mut global.remote_core, address),
        // Data service
        ConfigureSubCommand::Data { address } => process_field(&mut global.remote_data, address),
        // Frontend service
        ConfigureSubCommand::Frontend { address } => {
            process_field(&mut global.remote_frontend, address)
        }
    };
    // Save the config
    global.to_disk()?;
    // Ok
    result
}
