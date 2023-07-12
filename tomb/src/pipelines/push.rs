use crate::{
    pipelines::error::PipelineError, types::config::globalconfig::GlobalConfig,
    utils::wnfsio::get_progress_bar,
};
use anyhow::Result;
use std::path::Path;
use tomb_common::types::blockstore::networkblockstore::NetworkBlockStore;
use wnfs::{common::BlockStore as WnfsBlockStore, libipld::Cid};

/// Takes locally packed car file data and throws it onto a server
pub async fn pipeline(origin: &Path) -> Result<(), PipelineError> {
    info!("Sending blocks to remote server.");

    // Load the manifest
    let mut global = GlobalConfig::from_disk()?;
    let wrapping_key = global.wrapping_key_from_disk()?;

    if let Some(config) = global.get_bucket(origin) {
        let (metadata_forest, content_forest, root_dir, key_manager) =
            &mut config.get_all(&wrapping_key).await?;

        // Grab all Block CIDs
        let children: Vec<Cid> = vec![]; //config.content.get_all_cids();

        // Initialize the progress bar using the number of Nodes to process
        let progress_bar = get_progress_bar(children.len() as u64)?;

        info!("The loaded metadata has revealed a FileSystem with {} blocks. Sending these to the network now...", children.len());

        // Use the globally configured remote endpoint to create a NetworkBlockstore
        let remote = NetworkBlockStore::new(&global.remote)?;

        // For each child CID in the list
        for child in children {
            // Grab the bytes from the local store
            let bytes = config.content.get_block(&child).await?;

            // Throw those bytes onto the remote network
            remote
                .put_block(bytes.to_vec(), wnfs::libipld::IpldCodec::Raw)
                .await?;

            // Denote progress for each loop iteration
            progress_bar.inc(1);
        }

        info!("🎉 Nice! A copy of this encrypted filesystem now sits at the remote instance you pointed it to.");

        // config
        //     .set_all(
        //         &wrapping_key,
        //         metadata_forest,
        //         content_forest,
        //         root_dir,
        //         key_manager,
        //     )
        //     .await?;

        global.update_config(&config)?;
        global.to_disk()?;

        Ok(())
    } else {
        Err(PipelineError::Uninitialized)
    }
}
