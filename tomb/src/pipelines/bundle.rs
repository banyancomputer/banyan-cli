use super::error::TombError;
use crate::cli::command::BucketSpecifier;
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
/// * `output_dir` - &Path representing the relative path of where to store the bundleed data.
/// * `manifest_file` - &Path representing the relative path of where to store the manifest file.
/// * `chunk_size` - The maximum size of a bundleed file / chunk in bytes.
/// * `follow_links` - Whether or not to follow symlinks when bundleing.
///
/// # Return Type
/// Returns `Ok(())` on success, otherwise returns an error.
pub async fn pipeline(
    global: &mut GlobalConfig,
    bucket_specifier: &BucketSpecifier,
    follow_links: bool,
) -> Result<String, TombError> {
    let wrapping_key = global.clone().wrapping_key().await?;
    let mut config = global.get_bucket_by_specifier(bucket_specifier)?;
    // Create bundleing plan
    let bundleing_plan = create_plans(&config.origin, follow_links).await?;
    // TODO: optionally turn off the progress bar
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = &get_progress_bar(bundleing_plan.len() as u64)?;
    let fs = &mut config.unlock_fs(&wrapping_key).await?;
    // Create a new delta for this bundleing operation
    config.content.add_delta()?;

    // Process all of the BundlePipelinePlans
    process_plans(
        &config.metadata,
        &config.content,
        &mut fs.metadata_forest,
        &mut fs.content_forest,
        &mut fs.root_dir,
        bundleing_plan,
        progress_bar,
    )
    .await?;

    config.save_fs(fs).await?;

    global.update_config(&config)?;
    global.to_disk()?;

    Ok(format!(
        "successfully bundled data into {}",
        config.origin.display()
    ))
}
