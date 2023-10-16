use super::error::TombError;
use crate::types::config::bucket::LocalBucket;
use crate::utils::wnfsio::get_progress_bar;
use crate::{
    types::config::globalconfig::GlobalConfig,
    utils::bundle::{create_plans, process_plans},
};
use anyhow::Result;
/// Given the input directory, the output directory, the manifest file, and other metadata,
/// bundle the input directory into the output directory and store a record of how this
/// operation was performed in the manifest file.
///
/// # Arguments
///
/// * `input_dir` - &Path representing the relative path of the input directory to bundle.
/// * `output_dir` - &Path representing the relative path of where to store the bundled data.
/// * `manifest_file` - &Path representing the relative path of where to store the manifest file.
/// * `chunk_size` - The maximum size of a bundled file / chunk in bytes.
/// * `follow_links` - Whether or not to follow symlinks when bundling.
///
/// # Return Type
/// Returns `Ok(())` on success, otherwise returns an error.
pub async fn pipeline(
    global: &mut GlobalConfig,
    mut local: LocalBucket,
    follow_links: bool,
) -> Result<String, TombError> {
    let wrapping_key = global.wrapping_key().await?;
    let mut fs = local.unlock_fs(&wrapping_key).await?;

    // Create bundling plan
    let bundling_plan = create_plans(&local.origin, follow_links).await?;
    // TODO: optionally turn off the progress bar
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = get_progress_bar(bundling_plan.len() as u64)?;
    // Create a new delta for this bundling operation
    local.content.add_delta()?;

    // Process all of the BundlePipelinePlans
    process_plans(
        &mut fs,
        bundling_plan,
        &local.metadata,
        &local.content,
        &progress_bar,
    )
    .await?;

    local.save_fs(&mut fs).await?;

    global.update_config(&local)?;
    global.to_disk()?;

    Ok(format!(
        "successfully bundled data into {}",
        local.origin.display()
    ))
}
