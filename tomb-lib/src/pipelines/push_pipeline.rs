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
    let tomb_path = input_dir.join(".tomb");
    let content_path = input_dir.join("content");

    // Load the manifest
    let mut manifest = load_manifest(&tomb_path).await?;

    // Update the locations of the CarBlockStores to be relative to the input path
    manifest.meta_store.change_dir(&tomb_path)?;
    manifest.content_store.change_dir(&content_path)?;

    // Grab all Block CIDs
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

#[cfg(test)]
mod test {
    use std::{
        net::Ipv4Addr,
        path::{Path, PathBuf},
    };

    use anyhow::Result;
    use fake_file::{Strategy, Structure};

    use crate::{
        pipelines::{pack_pipeline::pack_pipeline, push_pipeline::push_pipeline},
        types::blockstore::networkblockstore::NetworkBlockStore, utils::fs::ensure_path_exists_and_is_empty_dir,
    };

    // Set up temporary filesystem for test cases
    async fn setup() -> Result<(PathBuf, PathBuf, PathBuf)> {
        // Base of the test directory
        let root_path = PathBuf::from("test").join("push");
        // Create and empty the dir
        ensure_path_exists_and_is_empty_dir(&root_path, true)?;
        // Input and output paths
        let input_path = root_path.join("input");
        let output_path = root_path.join("output");
        // Generate file structure
        Structure::new(2, 2, 1048576, Strategy::Simple).generate(&input_path)?;
        // Return all paths
        Ok((root_path, input_path, output_path))
    }

    // Remove contents of temporary dir
    async fn teardown(root_path: &Path) -> Result<()> {
        Ok(std::fs::remove_dir_all(root_path)?)
    }

    #[tokio::test]
    async fn test_push() -> Result<()> {
        // Create the setup conditions
        let (root_path, input_dir, output_dir) = setup().await?;
        pack_pipeline(&input_dir, &output_dir, 262144, true).await?;

        // Construct NetworkBlockStore and run pipeline
        let store = NetworkBlockStore::new(Ipv4Addr::new(127, 0, 0, 1), 5001);
        push_pipeline(&output_dir, &store).await?;

        // Teardown
        teardown(&root_path).await
    }
}
