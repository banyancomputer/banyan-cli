use std::{collections::HashSet, fs::remove_file, path::Path, rc::Rc};

use anyhow::Result;
use tomb_common::types::{
    blockstore::{car::carv2::blockstore::BlockStore, networkblockstore::NetworkBlockStore},
    config::globalconfig::GlobalConfig,
};
use wnfs::{common::BlockStore as WnfsBlockStore, private::PrivateForest};

use crate::utils::wnfsio::get_progress_bar;

use super::error::PipelineError;

/// Takes locally packed car file data and throws it onto a server
pub async fn pipeline(origin: &Path) -> Result<(), PipelineError> {
    let mut global = GlobalConfig::from_disk()?;

    if let Some(mut config) = global.get_bucket(origin) {
        // Overwrite the content with new BlockStore
        remove_file(&config.content.path).ok();
        config.content = BlockStore::new(&config.content.path)?;

        // Load
        let (metadata_forest, content_forest, root_dir, key_manager) =
            &mut config.get_all().await?;

        // TODO (organizedgrime) submit a pull request on WNFS to make this simpler. This is so clunky.
        // Find CID differences as a way of tallying all Forest CIDs
        let differences = content_forest
            .diff(&Rc::new(PrivateForest::new()), &config.content)
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

        let remote = NetworkBlockStore::new(&global.remote)?;

        // For each CID found
        for child in children {
            // Grab the bytes from the remote network
            let bytes = remote.get_block(&child).await?;
            // Throw those bytes onto the local store
            config
                .content
                .put_block(bytes.to_vec(), wnfs::libipld::IpldCodec::Raw)
                .await?;
            // Denote progress for each loop iteration
            progress_bar.inc(1);
        }

        info!("🎉 Nice! A copy of the remote encrypted filesystem now exists locally.");

        config
            .set_all(metadata_forest, content_forest, root_dir, key_manager)
            .await?;

        // Store the modified cold forest both locally and remotely
        global.update_config(&config)?;
        global.to_disk()?;

        Ok(())
    } else {
        Err(PipelineError::Uninitialized)
    }
}
