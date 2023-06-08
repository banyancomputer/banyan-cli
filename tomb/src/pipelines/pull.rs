use anyhow::Result;
use std::{collections::HashSet, path::Path, rc::Rc};
use wnfs::{common::BlockStore, private::PrivateForest};

use crate::utils::{
    fs::ensure_path_exists_and_is_empty_dir,
    serialize::{load_manifest, store_manifest},
    wnfsio::get_progress_bar,
};
use tomb_common::{types::blockstore::{
    carblockstore::CarBlockStore, networkblockstore::NetworkBlockStore,
}, utils::serialize::{load_cold_forest, store_cold_forest}};

/// Takes locally packed car file data and throws it onto a server
pub async fn pipeline(dir: &Path) -> Result<()> {
    // Represent relative directories for .tomb and content
    let tomb_path = dir.join(".tomb");
    let content_path = dir.join("content");
    // Load the Manifest
    let mut manifest = load_manifest(&tomb_path)?;
    // If this remote endpoint has not actually been configured
    if manifest.cold_remote == NetworkBlockStore::default() {
        panic!("Configure the remote endpoint for this filesystem using tomb config remote before running this command");
    }

    // // Ensure that there is a .tomb path
    // ensure_path_exists_and_is_dir(&tomb_path)?;
    // Empty the packed contents if there are any
    ensure_path_exists_and_is_empty_dir(&content_path, true)?;

    // // Update the locations of the CarBlockStores to be relative to the input path
    // manifest.hot_local.change_dir(&tomb_path)?;

    // Erase the old content store, assume all data has been lost
    manifest.cold_local = CarBlockStore::new(&content_path, None);

    // Load the cold forest from the remote endpoint
    let cold_forest = load_cold_forest(&manifest.roots, &manifest.cold_remote).await?;

    // TODO (organizedgrime) submit a pull request on WNFS to make this simpler. This is so clunky.
    // Find CID differences as a way of tallying all Forest CIDs
    let differences = cold_forest
        .diff(&Rc::new(PrivateForest::new()), &manifest.cold_local)
        .await?;

    let mut children = HashSet::new();
    for difference in differences {
        if let Some(difference1) = difference.value1 {
            children.extend(difference1);
        }
    }

    // TODO: optionally turn off the progress bar
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = get_progress_bar(children.len() as u64)?;
    // For each CID found
    for child in children {
        // Grab the bytes from the remote network
        let bytes = manifest.cold_remote.get_block(&child).await?;
        // Throw those bytes onto the local store
        manifest
            .cold_local
            .put_block(bytes.to_vec(), wnfs::libipld::IpldCodec::Raw)
            .await?;
        // Denote progress for each loop iteration
        progress_bar.inc(1);
    }

    info!("🎉 Nice! A copy of the remote encrypted filesystem now exists locally.");

    // Store the modified cold forest both locally and remotely
    store_cold_forest(&mut manifest.roots, &manifest.cold_local, &cold_forest).await?;
    store_cold_forest(&mut manifest.roots, &manifest.cold_remote, &cold_forest).await?;
    // Store the modified manifest
    store_manifest(&tomb_path, &manifest)
}
