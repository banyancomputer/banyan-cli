use std::path::Path;

use anyhow::Result;

/// Takes locally packed car file data and throws it onto a server
pub async fn pipeline(_dir: &Path) -> Result<()> {
    // // Represent relative directories for .tomb and content
    // let tomb_path = dir.join(".tomb");
    // let content_path = dir.join("content.car");
    // // Load the Manifest
    // let mut manifest = manifest_from_disk(&tomb_path)?;
    // // If this remote endpoint has not actually been configured
    // if manifest.cold_remote == NetworkBlockStore::default() {
    //     panic!("Configure the remote endpoint for this filesystem using tomb config remote before running this command");
    // }

    // // // Update the locations of the CarV2BlockStores to be relative to the input path
    // // manifest.metadata.change_dir(&tomb_path)?;

    // // Erase the old content store, assume all data has been lost
    // manifest.content = CarV2BlockStore::new(&content_path)?;

    // // Load the cold forest from the remote endpoint
    // let content_forest = load_content_forest(&manifest.roots, &manifest.cold_remote).await?;

    // // TODO (organizedgrime) submit a pull request on WNFS to make this simpler. This is so clunky.
    // // Find CID differences as a way of tallying all Forest CIDs
    // let differences = content_forest
    //     .diff(&Rc::new(PrivateForest::new()), &manifest.content)
    //     .await?;

    // let mut children = HashSet::new();
    // for difference in differences {
    //     if let Some(difference1) = difference.value1 {
    //         children.extend(difference1);
    //     }
    // }

    // // TODO: optionally turn off the progress bar
    // // Initialize the progress bar using the number of Nodes to process
    // let progress_bar = get_progress_bar(children.len() as u64)?;
    // // For each CID found
    // for child in children {
    //     // Grab the bytes from the remote network
    //     let bytes = manifest.cold_remote.get_block(&child).await?;
    //     // Throw those bytes onto the local store
    //     manifest
    //         .content
    //         .put_block(bytes.to_vec(), wnfs::libipld::IpldCodec::Raw)
    //         .await?;
    //     // Denote progress for each loop iteration
    //     progress_bar.inc(1);
    // }

    // info!("🎉 Nice! A copy of the remote encrypted filesystem now exists locally.");

    // // Store the modified cold forest both locally and remotely
    // store_content_forest(&mut manifest.roots, &manifest.content, &content_forest).await?;
    // store_content_forest(&mut manifest.roots, &manifest.cold_remote, &content_forest).await?;
    // // Store the modified manifest
    // manifest_to_disk(&tomb_path, &manifest)
    Ok(())
}
