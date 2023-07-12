use crate::{
    types::config::globalconfig::GlobalConfig,
    utils::{
        pack::{create_plans, process_plans, store_keys, update_key_content},
        wnfsio::get_progress_bar,
    },
};
use anyhow::Result;
use chrono::Utc;
use log::info;
use rand::thread_rng;
use std::{path::Path, rc::Rc};
use tomb_common::{types::{keys::manager::Manager, blockstore::tombblockstore::TombBlockStore}, utils::serialize::{store_dirs_update_keys, store_forests, store_all}};
use wnfs::{
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateForest}, libipld::IpldCodec, common::{dagcbor, BlockStore},
};

use super::error::PipelineError;

/// Given the input directory, the output directory, the manifest file, and other metadata,
/// pack the input directory into the output directory and store a record of how this
/// operation was performed in the manifest file.
///
/// # Arguments
///
/// * `input_dir` - &Path representing the relative path of the input directory to pack.
/// * `output_dir` - &Path representing the relative path of where to store the packed data.
/// * `manifest_file` - &Path representing the relative path of where to store the manifest file.
/// * `chunk_size` - The maximum size of a packed file / chunk in bytes.
/// * `follow_links` - Whether or not to follow symlinks when packing.
///
/// # Return Type
/// Returns `Ok(())` on success, otherwise returns an error.
pub async fn pipeline(
    origin: &Path,
    // _chunk_size: u64,
    follow_links: bool,
) -> Result<(), PipelineError> {
    // Create packing plan
    let packing_plan = create_plans(origin, follow_links).await?;
    // TODO: optionally turn off the progress bar
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = &get_progress_bar(packing_plan.len() as u64)?;

    let mut global = GlobalConfig::from_disk()?;
    let wrapping_key = global.wrapping_key_from_disk()?;

    // If the user has done initialization for this directory
    if let Some(mut config) = global.get_bucket(origin) {
        // Create the root directory in which all Nodes will be stored
        let mut root_dir = Rc::new(PrivateDirectory::new(
            Namefilter::default(),
            Utc::now(),
            &mut thread_rng(),
        ));
        // Create the PrivateForest from which Nodes will be queried
        let mut metadata_forest = Rc::new(PrivateForest::new());
        let mut content_forest = Rc::new(PrivateForest::new());

        let mut manager = Manager::default();

        // If this filesystem has already been packed
        if let Ok((new_metadata_forest, new_content_forest, new_root_dir, new_manager)) =
            config.get_all(&wrapping_key).await
        {
            // Update structs
            metadata_forest = new_metadata_forest;
            content_forest = new_content_forest;
            root_dir = new_root_dir;
            manager = new_manager;
        } else {
            info!("tomb has not seen this filesystem before, starting from scratch! 💖");
        }

        // Create a new delta for this packing operation
        config.content.add_delta()?;
        // Insert the wrapping key if it is not already there
        manager.insert(&wrapping_key.get_public_key()).await?;
        // Put the keys in the BlockStores before any other data
        let manager_cid = store_keys(&manager, &config.metadata, &config.content).await?;
        
        // Process all of the PackPipelinePlans
        process_plans(
            packing_plan,
            progress_bar,
            &mut root_dir,
            &mut metadata_forest,
            &mut content_forest,
            &config.metadata,
            &config.content,
        )
        .await?;

        // Store dirs, update keys
        let (original_ref_cid, current_ref_cid) = store_dirs_update_keys(&config.metadata, &config.content, &mut metadata_forest, &mut content_forest, &root_dir, &mut manager).await?;
        // Store forests 
        let (metadata_forest_cid, content_forest_cid) = store_forests(&config.metadata, &config.content, &mut metadata_forest, &mut content_forest).await?;
        // Update content for Key Manager
        let manager_cid = update_key_content(&manager, manager_cid, &config.metadata, &config.content).await?;

        // Store everything
        store_all(&config.metadata, &config.content, original_ref_cid, current_ref_cid, metadata_forest_cid, content_forest_cid, manager_cid).await?;
        global.update_config(&config)?;
        global.to_disk()?;
        Ok(())
    } else {
        Err(PipelineError::Uninitialized)
    }
}
