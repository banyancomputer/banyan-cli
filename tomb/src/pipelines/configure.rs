use crate::types::config::globalconfig::GlobalConfig;
use anyhow::Result;
use std::path::Path;
use tomb_common::banyan_api::client::Client;

/// Create a default config for this user
pub async fn init(path: &Path) -> Result<()> {
    let mut global = GlobalConfig::from_disk().await?;
    global.new_bucket(path)?;
    global.to_disk().await
}

/// Remove all configuration data for a given bucket
pub async fn deinit(path: &Path) -> Result<()> {
    let mut global = GlobalConfig::from_disk().await?;
    global.remove(path)?;
    global.to_disk().await
}

/// Remove all configuration data
pub async fn deinit_all() -> Result<()> {
    GlobalConfig::from_disk().await?.remove_data()
}

/// Configure the remote endpoint in a given directory, assuming initializtion has already taken place
pub async fn remote(address: &str) -> Result<()> {
    let mut config = GlobalConfig::from_disk().await?;
    config.client = Some(Client::new(address)?);
    config.to_disk().await
}
