use anyhow::Result;
use std::{env, fs::create_dir_all, net::Ipv4Addr, path::PathBuf};

/// Grab config path from env variables + XDG spec
pub fn tomb_config() -> Result<PathBuf> {
    // Construct
    let path = PathBuf::from(format!("{}/.config/tomb", env::var("HOME")?));
    // If the directory doesnt exist yet, make it!
    create_dir_all(&path)?;
    // Return
    Ok(path)
}

/// Helper function for creating the required type
/// TODO(organizedgrime) - ipv4 sucks. switch to urls soon.
pub fn ip_from_string(address: String) -> Ipv4Addr {
    // Represent the string as an array of four numbers exactly
    let numbers: [u8; 4] = address
        .split('.')
        .map(|s| s.parse::<u8>().unwrap())
        .collect::<Vec<u8>>()
        .as_slice()
        .try_into()
        .expect("IP Address was not formatted correctly");

    // Construct the IP Address from these numbers
    Ipv4Addr::from(numbers)
}
