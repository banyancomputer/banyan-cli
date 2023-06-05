use crate::utils::{
    fs::ensure_path_exists_and_is_empty_dir,
    serialize::{load_manifest, store_manifest},
};
use anyhow::Result;
use std::{fs::create_dir_all, path::Path};
use tomb_common::types::{blockstore::networkblockstore::NetworkBlockStore, pipeline::Manifest};

/// Initialize the .tomb metadata directory and Manifest file therein
pub fn init(dir: &Path) -> Result<()> {
    // Append the expected .tomb
    let tomb_path = &dir.join(".tomb");
    // Remove existing metadata
    // if tomb_path.exists() {
    //     let mut valid_response = false;
    //     let mut response = String::new();
    //     while !valid_response {
    //         info!("You've already initialized tomb in this directory, are you sure you want to erase metadata from previous runs by reinitializing? Y/n");
    //         stdin().read_line(&mut response)?;
    //         response = response.to_ascii_lowercase();
    //         if &response == "y" || &response == "n" {
    //             valid_response = true;
    //         }
    //     }

    //     // If we don't care about that which was already there
    //     if &response == "y" {
    //         remove_dir_all(tomb_path)?;
    //         // Create new metadata folder
    //         create_dir_all(tomb_path)?;

    //     }
    // }
    //
    create_dir_all(tomb_path)?;
    // Forcibly ensure the path exists and is empty
    ensure_path_exists_and_is_empty_dir(tomb_path, true)?;
    // Store the default / empty manifest
    store_manifest(tomb_path, &Manifest::default())
}

/// Configure the remote endpoint in a given directory, assuming initializtion has already taken place
pub fn remote(dir: &Path, url: &str, port: u16) -> Result<()> {
    // Append the expected .tomb
    let tomb_path = &dir.join(".tomb");
    let mut manifest = load_manifest(tomb_path)?;
    // Set the remote endpoint
    manifest.cold_remote = NetworkBlockStore::new(url, port);
    // Store the updated Manifest
    store_manifest(tomb_path, &manifest)
}
