use anyhow::Result;
use fake_file::utils::ensure_path_exists_and_is_dir;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    collections::HashSet,
    path::Path,
    rc::Rc,
    sync::{Arc, Mutex},
};
use wnfs::{common::BlockStore, private::PrivateForest};

use crate::utils::{
    fs::ensure_path_exists_and_is_empty_dir,
    serialize::{load_forest, load_manifest, store_manifest},
};
use tomb_common::types::blockstore::{
    carblockstore::CarBlockStore, networkblockstore::NetworkBlockStore,
};

/// Takes locally packed car file data and throws it onto a server
pub async fn pipeline(dir: &Path, store: &NetworkBlockStore) -> Result<()> {
    info!("Downloading blocks from remote server.");

    // Represent relative directories for .tomb and content
    let tomb_path = dir.join(".tomb");
    let content_path = dir.join("content");
    // Ensure that there is a .tomb path
    ensure_path_exists_and_is_dir(&tomb_path)?;
    // Empty the packed contents if there are any
    ensure_path_exists_and_is_empty_dir(&content_path, true)?;

    // Load the Manifest
    let mut manifest = load_manifest(&tomb_path)?;

    // Update the locations of the CarBlockStores to be relative to the input path
    manifest.meta_store.change_dir(&tomb_path)?;

    // Erase the old content store, assume all data has been lost
    manifest.content_store = CarBlockStore::new(&content_path, None);

    // Load the forest
    let forest = load_forest(&manifest).await?;

    // TODO (organizedgrime) submit a pull request on WNFS to make this simpler. This is so clunky.
    // Find CID differences as a way of tallying all Forest CIDs
    let differences = forest
        .diff(&Rc::new(PrivateForest::new()), &manifest.content_store)
        .await?;
    let mut children = HashSet::new();
    for difference in differences {
        if let Some(difference1) = difference.value1 {
            children.extend(difference1);
        }
        if let Some(difference2) = difference.value2 {
            children.extend(difference2);
        }
    }

    // TODO: optionally turn off the progress bar
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = ProgressBar::new(children.len() as u64);
    // Stylize that progress bar!
    progress_bar.set_style(ProgressStyle::default_bar().template(
        "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )?);
    // Create a usable instance of the progress bar by wrapping the obj in Mutex and Arc
    let progress_bar = Arc::new(Mutex::new(progress_bar));

    for child in children {
        // Grab the bytes from the remote network
        let bytes = store.get_block(&child).await?;
        // Throw those bytes onto the local store
        manifest
            .content_store
            .put_block(bytes.to_vec(), wnfs::libipld::IpldCodec::Raw)
            .await?;
        // Denote progress for each loop iteration
        progress_bar.lock().unwrap().inc(1);
    }

    info!("🎉 Nice! A copy of the remote encrypted filesystem now exists locally.");

    // Store the modified manifest
    store_manifest(&tomb_path, &manifest)
}
