use std::path::PathBuf;

use super::error::TombError;
use crate::types::config::bucket::LocalBucket;
use crate::types::spider::PreparePipelinePlan;
use crate::utils::wnfsio::get_progress_bar;
use crate::{
    types::config::globalconfig::GlobalConfig,
    utils::prepare::{create_plans, process_plans},
};
use anyhow::Result;
use tomb_common::utils::wnfsio::path_to_segments;
use wnfs::private::PrivateNode;
/// Given the input directory, the output directory, the manifest file, and other metadata,
/// prepare the input directory into the output directory and store a record of how this
/// operation was performed in the manifest file.
///
/// # Arguments
///
/// * `input_dir` - &Path representing the relative path of the input directory to prepare.
/// * `output_dir` - &Path representing the relative path of where to store the prepared data.
/// * `manifest_file` - &Path representing the relative path of where to store the manifest file.
/// * `chunk_size` - The maximum size of a prepared file / chunk in bytes.
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

    // Get all the paths present on disk
    let mut all_disk_paths = <Vec<PathBuf>>::new();
    for plan in bundling_plan.clone() {
        match plan {
            PreparePipelinePlan::Directory(metadata)
            | PreparePipelinePlan::Symlink(metadata, _) => {
                all_disk_paths.push(metadata.original_location.clone());
            }
            PreparePipelinePlan::FileGroup(metadatas) => {
                let paths: Vec<PathBuf> = metadatas
                    .iter()
                    .map(|metadata| metadata.original_location.clone())
                    .collect();
                all_disk_paths.extend(paths);
            }
        }
    }

    // Get all nodes and their associated paths
    let all_node_paths = fs.get_all_nodes(&local.metadata).await?;

    // Track all blocks removed since the last preparation
    for (node, wnfs_path) in all_node_paths {
        // If the existing WNFS node is not still represented on disk
        if !all_disk_paths.contains(&wnfs_path) {
            // If the node is a File, add all the CIDs associated with it to a list
            if let PrivateNode::File(file) = node {
                local
                    .deleted_blocks
                    .extend(file.get_cids(&fs.forest, &local.metadata).await?);
            }
            // Remove the reference from the WNFS
            fs.rm(&path_to_segments(&wnfs_path)?, &local.metadata)
                .await?;
        }
    }

    // TODO: optionally turn off the progress bar
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = get_progress_bar(bundling_plan.len() as u64)?;
    // Create a new delta for this bundling operation
    local.content.add_delta()?;

    // Process all of the PreparePipelinePlans
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
        "successfully prepared data into {}",
        local.origin.display()
    ))
}
