use crate::utils::{
    pack::{create_plans, process_plans},
    wnfsio::get_progress_bar,
};
use anyhow::Result;
use chrono::Utc;
use log::info;
use rand::thread_rng;
use std::{path::Path, rc::Rc};
use tomb_common::types::config::{globalconfig::GlobalConfig, keys::manager::Manager};
use wnfs::{
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateForest},
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
    if let Some(config) = global.get_bucket(origin) {
        // Create the root directory in which all Nodes will be stored
        let mut root_dir = Rc::new(PrivateDirectory::new(
            Namefilter::default(),
            Utc::now(),
            &mut thread_rng(),
        ));
        // Create the PrivateForest from which Nodes will be queried
        let mut metadata_forest = Rc::new(PrivateForest::new());
        let mut content_forest = Rc::new(PrivateForest::new());

        let mut key_manager = Manager::default();

        // If this filesystem has already been packed
        if let Ok((new_metadata_forest, new_content_forest, new_root_dir, new_key_manager)) =
            config.get_all(&wrapping_key).await
        {
            // Update structs
            metadata_forest = new_metadata_forest;
            content_forest = new_content_forest;
            root_dir = new_root_dir;
            key_manager = new_key_manager;
            println!(
                "zomg! have seen this before: \n[[[[\n{:?}\n]]]]",
                key_manager
            );
        } else {
            info!("tomb has not seen this filesystem before, starting from scratch! 💖");
        }

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

        // Store Forest and Dir in BlockStores and Key
        config
            .set_all(
                &wrapping_key,
                &mut metadata_forest,
                &mut content_forest,
                &root_dir,
                &mut key_manager,
            )
            .await?;

        global.update_config(&config)?;
        global.to_disk()?;
        Ok(())
    } else {
        Err(PipelineError::Uninitialized)
    }
}
