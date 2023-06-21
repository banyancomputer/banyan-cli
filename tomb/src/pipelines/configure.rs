use anyhow::Result;
use std::path::Path;
use tomb_common::types::config::globalconfig::GlobalConfig;

/// Create a default config for this user
pub fn init(path: &Path) -> Result<()> {
    let mut global = GlobalConfig::from_disk()?;
    global.new_bucket(path)?;
    global.to_disk()
}

/// Remove all configuration data for a given bucket
pub fn deinit(path: &Path) -> Result<()> {
    let mut global = GlobalConfig::from_disk()?;
    global.remove(path)?;
    global.to_disk()
}

/// Remove all configuration data
pub fn deinit_all() -> Result<()> {
    GlobalConfig::from_disk()?.remove_data()
}

/// Configure the remote endpoint in a given directory, assuming initializtion has already taken place
pub fn remote(_url: &str, _port: u16) -> Result<()> {
    // let mut config = GlobalConfig::from_disk()?;
    // config.remote = format!("{}:{}", url, port);
    // config.to_disk()
    Ok(())
}
