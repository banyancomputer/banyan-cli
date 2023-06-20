use anyhow::Result;
use std::path::Path;
use tomb_common::types::config::globalconfig::GlobalConfig;

/// Create a default config for this user
pub fn init(path: &Path) -> Result<()> {
    GlobalConfig::new_bucket(path)?;
    Ok(())
}

/// Remove all configuration data for this user
pub fn deinit(path: &Path) -> Result<()> {
    GlobalConfig::remove(path)
}

/// Configure the remote endpoint in a given directory, assuming initializtion has already taken place
pub fn remote(url: &str, port: u16) -> Result<()> {
    // let mut config = GlobalConfig::from_disk()?;
    // config.remote = format!("{}:{}", url, port);
    // config.to_disk()
    Ok(())
}
