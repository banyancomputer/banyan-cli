use crate::{
    types::config::globalconfig::GlobalConfig,
    utils::{
        pack::{create_plans, process_plans},
        wnfsio::get_progress_bar,
    },
};
use anyhow::Result;
use std::path::Path;
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
    follow_links: bool,
) -> Result<(), PipelineError> {
    // Create packing plan
    let packing_plan = create_plans(origin, follow_links).await?;
    // TODO: optionally turn off the progress bar
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = &get_progress_bar(packing_plan.len() as u64)?;

    let mut global = GlobalConfig::from_disk().await?;
    let wrapping_key = global.clone().wrapping_key().await?;

    // If the user has done initialization for this directory
    if let Some(mut config) = global.get_bucket(origin) {
        let (
            metadata_forest,
            content_forest,
            root_dir,
            manager,
        ) = &mut config.get_all(&wrapping_key).await?;
        // Create a new delta for this packing operation
        config.content.add_delta()?;

        // Process all of the PackPipelinePlans
        process_plans(
            &config.metadata,
            &config.content,
            metadata_forest,
            content_forest,
            root_dir,
            packing_plan,
            progress_bar,
        )
        .await?;

        config
            .set_all(
                metadata_forest,
                content_forest,
                root_dir,
                manager,
            )
            .await?;

        global.update_config(&config)?;
        global.to_disk()?;
        Ok(())
    } else {
        Err(PipelineError::Uninitialized)
    }
}
