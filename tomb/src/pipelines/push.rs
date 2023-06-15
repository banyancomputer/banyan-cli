use anyhow::Result;
use std::path::Path;

/// Takes locally packed car file data and throws it onto a server
pub async fn pipeline(_dir: &Path) -> Result<()> {
    // info!("Sending blocks to remote server.");
    // let tomb_path = dir.join(".tomb");
    // // let _content_path = dir.join("content");

    // // Load the manifest
    // let manifest = manifest_from_disk(&tomb_path)?;
    // info!("Loaded manifest...");

    // // Update the locations of the CarV2BlockStores to be relative to the input path
    // // manifest.metadata.change_dir(&tomb_path)?;
    // // manifest.content.change_dir(&content_path)?;
    // // manifest.cold_remote.addr = "".to_string();

    // // Grab all Block CIDs
    // let children: Vec<Cid> = manifest.content.get_all_cids();

    // // Initialize the progress bar using the number of Nodes to process
    // let progress_bar = get_progress_bar(children.len() as u64)?;

    // info!("The loaded metadata has revealed a FileSystem with {} blocks. Sending these to the network now...", children.len());

    // // For each child CID in the list
    // for child in children {
    //     // Grab the bytes from the local store
    //     let bytes = manifest.content.get_block(&child).await?;
    //     // Throw those bytes onto the remote network
    //     manifest
    //         .cold_remote
    //         .put_block(bytes.to_vec(), wnfs::libipld::IpldCodec::Raw)
    //         .await?;

    //     // Denote progress for each loop iteration
    //     progress_bar.inc(1);
    // }

    // info!("🎉 Nice! A copy of this encrypted filesystem now sits at the remote instance you pointed it to.");

    Ok(())
}
