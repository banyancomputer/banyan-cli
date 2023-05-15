use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use wnfs::{common::BlockStore, libipld::Cid};

use crate::{
    types::blockstore::networkblockstore::NetworkBlockStore, utils::pipeline::load_manifest,
};

/// Takes locally packed car file data and throws it onto a server
pub async fn push_pipeline(input_dir: &Path, store: &NetworkBlockStore) -> Result<()> {
    let input_meta_path = input_dir.join(".tomb");
    let manifest = load_manifest(&input_meta_path).await?;
    let children: Vec<Cid> = manifest.content_store.get_all_cids();

    // TODO: optionally turn off the progress bar
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = ProgressBar::new(children.len() as u64);
    // Stylize that progress bar!
    progress_bar.set_style(ProgressStyle::default_bar().template(
        "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )?);
    // Create a usable instance of the progress bar by wrapping the obj in Mutex and Arc
    let progress_bar = Arc::new(Mutex::new(progress_bar));

    info!("The loaded metadata has revealed a FileSystem with {} blocks. Sending these to the network now...", children.len());

    // For each child CID in the list
    for child in children {
        // Grab the bytes from the local store
        let bytes = manifest.content_store.get_block(&child).await?;
        // Throw those bytes onto the remote network
        store
            .put_block(bytes.to_vec(), wnfs::libipld::IpldCodec::Raw)
            .await?;

        // Denote progress for each loop iteration
        progress_bar.lock().unwrap().inc(1);
    }

    info!("🎉 Nice! A copy of this encrypted filesystem now sits at the remote instance you pointed it to.");

    Ok(())
}
