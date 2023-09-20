use anyhow::Result;
use indicatif::ProgressBar;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    fs::File, io::Read,
};

use super::spider::path_to_segments;
use crate::{
    types::spider::BundlePipelinePlan,
    utils::{grouper::grouper, spider},
};
use tomb_common::{metadata::FsMetadata, blockstore::RootedBlockStore};

/// Create BundlePipelinePlans from an origin dir
pub async fn create_plans(origin: &Path, follow_links: bool) -> Result<Vec<BundlePipelinePlan>> {
    // HashSet to track files that have already been seen
    let mut seen_files: HashSet<PathBuf> = HashSet::new();
    // Vector holding all the BundlePipelinePlans for bundleing
    let mut bundleing_plan: Vec<BundlePipelinePlan> = vec![];

    info!("🔍 Deduplicating the filesystem at {}", origin.display());
    // Group the filesystem provided to detect duplicates
    let group_plans = grouper(origin, follow_links, &mut seen_files)?;
    // Extend the bundleing plan
    bundleing_plan.extend(group_plans);

    // TODO fix setting follow_links / do it right
    info!(
        "📁 Finding directories and symlinks to back up starting at {}",
        origin.display()
    );

    // Spider the filesystem provided to include directories and symlinks
    let spidered_files = spider::spider(origin, follow_links, &mut seen_files).await?;
    // Extend the bundleing plan
    bundleing_plan.extend(spidered_files);

    info!(
        "💾 Total number of files to bundle: {}",
        bundleing_plan.len()
    );

    Ok(bundleing_plan)
}

/// Given a set of BundlePipelinePlans and required structs, process each
pub async fn process_plans(
    fs: &mut FsMetadata,
    bundling_plan: Vec<BundlePipelinePlan>,
    metadata_store: &impl RootedBlockStore,
    content_store: &impl RootedBlockStore,
    progress_bar: &ProgressBar,
) -> Result<()> {
    // Create vectors of direct and indirect plans
    let mut direct_plans: Vec<BundlePipelinePlan> = Vec::new();
    let mut symlink_plans: Vec<BundlePipelinePlan> = Vec::new();

    // Sort the bundleing plans into plans which correspond to real data and those which are symlinks
    for bundle_pipeline_plan in bundling_plan {
        match bundle_pipeline_plan.clone() {
            BundlePipelinePlan::FileGroup(_) | BundlePipelinePlan::Directory(_) => {
                direct_plans.push(bundle_pipeline_plan);
            }
            BundlePipelinePlan::Symlink(_, _) => {
                symlink_plans.push(bundle_pipeline_plan);
            }
        }
    }

    // First, write data which corresponds to real data
    for direct_plan in direct_plans {
        match direct_plan {
            BundlePipelinePlan::FileGroup(metadatas) => {
                // Grab the metadata for the first occurrence of this file
                let first = &metadatas
                    .first()
                    .expect("no metadatas present")
                    .original_location;
                // Turn the relative path into a vector of segments
                let path_segments = path_to_segments(first)?;
                // Load the file from disk
                let mut file = File::open(&metadatas.get(0).expect("no paths").canonicalized_path)?;
                let mut content = <Vec<u8>>::new();
                file.read_to_end(&mut content)?;
                // Add the file contents
                fs.add(path_segments, content, metadata_store, content_store).await?;

                // Duplicates need to be linked no matter what
                for meta in &metadatas[1..] {
                    // Grab the original location
                    let dup = &meta.original_location;
                    let dup_path_segments = &path_to_segments(dup)?;
                    // Remove the final element to represent the folder path
                    let folder_segments = &dup_path_segments[..&dup_path_segments.len() - 1];
                    // Create that folder
                    fs.mkdir(folder_segments.to_vec(), metadata_store).await?;
                    
                    
                    // TODO CP LINK DEDUPLICATION
                    // // Copy the file from the original path to the duplicate path
                    // root_dir
                    //     .cp_link(
                    //         path_segments,
                    //         dup_path_segments,
                    //         true,
                    //         forest,
                    //         metadata_store,
                    //     )
                    //     .await?;
                }
            }
            // If this is a directory or symlink
            BundlePipelinePlan::Directory(meta) => {
                // Turn the canonicalized path into a vector of segments
                let path_segments = path_to_segments(&meta.original_location)?;
                // If the directory does not exist
                if fs.get_node(path_segments.clone(), metadata_store).await.is_err() {
                    // Create the subdirectory
                    fs.mkdir(path_segments, metadata_store).await?;
                }
            }
            BundlePipelinePlan::Symlink(_, _) => panic!("this is unreachable code"),
        }

        // Denote progress for each loop iteration
        progress_bar.inc(1);
    }

    // Now that the data exists, we can symlink to it
    for symlink_plan in symlink_plans {
        match symlink_plan {
            BundlePipelinePlan::Symlink(meta, _symlink_target) => {
                // The path where the symlink will be placed
                let _symlink_segments = path_to_segments(&meta.original_location)?;



                // // Link the file or folder
                // root_dir
                //     .write_symlink(
                //         symlink_target
                //             .to_str()
                //             .expect("failed to represent as string")
                //             .to_string(),
                //         &symlink_segments,
                //         true,
                //         Utc::now(),
                //         forest,
                //         metadata_store,
                //         rng,
                //     )
                //     .await?;
            }
            BundlePipelinePlan::Directory(_) | BundlePipelinePlan::FileGroup(_) => {
                panic!("this is unreachable code")
            }
        }

        // Denote progress for each loop iteration
        progress_bar.inc(1);
    }

    // Return Ok
    Ok(())
}
