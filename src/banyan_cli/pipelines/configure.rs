use crate::banyan_cli::types::config::globalconfig::GlobalConfig;
use anyhow::Result;
use std::path::Path;

/// Create a default config for this user
pub async fn init(name: &str, path: &Path) -> Result<()> {
    let mut global = GlobalConfig::from_disk().await?;
    global.get_or_init_bucket(name, path).await?;
    global.to_disk()
}

/// Remove all configuration data for a given bucket
pub async fn deinit(path: &Path) -> Result<()> {
    let mut global = GlobalConfig::from_disk().await?;
    if let Some(local) = global.get_bucket(path) {
        global.remove_bucket(&local)?;
    }
    global.to_disk()
}

/// Remove all configuration data
pub async fn deinit_all() -> Result<()> {
    GlobalConfig::from_disk().await?.remove_data()
}

/// Configure the remote endpoint in a given directory, assuming initializtion has already taken place
pub async fn remote_core(address: &str) -> Result<String> {
    let mut config = GlobalConfig::from_disk().await?;
    config.endpoints.core = address.to_string();
    config.to_disk()?;
    Ok("saved remote address".to_string())
}

/// Configure the remote endpoint in a given directory, assuming initializtion has already taken place
pub async fn remote_data(address: &str) -> Result<String> {
    let mut config = GlobalConfig::from_disk().await?;
    config.endpoints.data = address.to_string();
    config.to_disk()?;
    Ok("saved remote address".to_string())
}