use crate::utils::{serialize::load_manifest, wnfsio::get_progress_bar};
use anyhow::Result;
use std::path::Path;
use tomb_common::types::blockstore::networkblockstore::NetworkBlockStore;
use wnfs::{common::BlockStore, libipld::Cid};

/// Takes locally packed car file data and throws it onto a server
pub async fn pipeline(dir: &Path) -> Result<()> {
    info!("Sending blocks to remote server.");
    let tomb_path = dir.join(".tomb");
    let content_path = dir.join("content");

    // Load the manifest
    let mut manifest = load_manifest(&tomb_path)?;

    // If this remote endpoint has not actually been configured
    if manifest.content_remote == NetworkBlockStore::default() {
        println!("content_remote: {:?}", manifest.content_remote);
        panic!("Configure the remote endpoint for this filesystem using tomb config remote before running this command");
    }

    // Update the locations of the CarBlockStores to be relative to the input path
    // manifest.meta_store.change_dir(&tomb_path)?;
    // manifest.content_local.change_dir(&content_path)?;
    // manifest.content_remote.addr = "".to_string();

    // Grab all Block CIDs
    let children: Vec<Cid> = manifest.content_local.get_all_cids();

    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = get_progress_bar(children.len() as u64)?;

    info!("The loaded metadata has revealed a FileSystem with {} blocks. Sending these to the network now...", children.len());

    // For each child CID in the list
    for child in children {
        // Grab the bytes from the local store
        let bytes = manifest.content_local.get_block(&child).await?;
        // Throw those bytes onto the remote network
        manifest
            .content_remote
            .put_block(bytes.to_vec(), wnfs::libipld::IpldCodec::Raw)
            .await?;

        // Denote progress for each loop iteration
        progress_bar.inc(1);
    }

    info!("🎉 Nice! A copy of this encrypted filesystem now sits at the remote instance you pointed it to.");

    Ok(())
}
