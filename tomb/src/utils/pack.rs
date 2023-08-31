use anyhow::Result;
use chrono::Utc;
use indicatif::ProgressBar;
use rand::thread_rng;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
};

use wnfs::{
    common::BlockStore as WnfsBlockStore,
    private::{PrivateDirectory, PrivateFile, PrivateForest},
};

use crate::{
    types::spider::PackPipelinePlan,
    utils::{grouper::grouper, spider},
};
use tomb_common::utils::wnfsio::compress_file;
use super::spider::path_to_segments;

/// Create PackPipelinePlans from an origin dir
pub async fn create_plans(origin: &Path, follow_links: bool) -> Result<Vec<PackPipelinePlan>> {
    // HashSet to track files that have already been seen
    let mut seen_files: HashSet<PathBuf> = HashSet::new();
    // Vector holding all the PackPipelinePlans for packing
    let mut packing_plan: Vec<PackPipelinePlan> = vec![];

    info!("🔍 Deduplicating the filesystem at {}", origin.display());
    // Group the filesystem provided to detect duplicates
    let group_plans = grouper(origin, follow_links, &mut seen_files)?;
    // Extend the packing plan
    packing_plan.extend(group_plans);

    // TODO fix setting follow_links / do it right
    info!(
        "📁 Finding directories and symlinks to back up starting at {}",
        origin.display()
    );

    // Spider the filesystem provided to include directories and symlinks
    let spidered_files = spider::spider(origin, follow_links, &mut seen_files).await?;
    // Extend the packing plan
    packing_plan.extend(spidered_files);

    info!("💾 Total number of files to pack: {}", packing_plan.len());

    Ok(packing_plan)
}

/// Given a set of PackPipelinePlans and required structs, process each
pub async fn process_plans(
    metadata: &impl WnfsBlockStore,
    content: &impl WnfsBlockStore,
    metadata_forest: &mut Rc<PrivateForest>,
    content_forest: &mut Rc<PrivateForest>,
    root_dir: &mut Rc<PrivateDirectory>,
    packing_plan: Vec<PackPipelinePlan>,
    progress_bar: &ProgressBar,
) -> Result<()> {
    // Rng
    let rng: &mut rand::rngs::ThreadRng = &mut thread_rng();
    // Create vectors of direct and indirect plans
    let mut direct_plans: Vec<PackPipelinePlan> = Vec::new();
    let mut symlink_plans: Vec<PackPipelinePlan> = Vec::new();

    // Sort the packing plans into plans which correspond to real data and those which are symlinks
    for pack_pipeline_plan in packing_plan {
        match pack_pipeline_plan.clone() {
            PackPipelinePlan::FileGroup(_) | PackPipelinePlan::Directory(_) => {
                direct_plans.push(pack_pipeline_plan);
            }
            PackPipelinePlan::Symlink(_, _) => {
                symlink_plans.push(pack_pipeline_plan);
            }
        }
    }

    // First, write data which corresponds to real data
    for direct_plan in direct_plans {
        match direct_plan {
            PackPipelinePlan::FileGroup(metadatas) => {
                // Grab the metadata for the first occurrence of this file
                let first = &metadatas
                    .first()
                    .expect("no metadatas present")
                    .original_location;
                // Turn the relative path into a vector of segments
                let path_segments = &path_to_segments(first)?;
                // Grab the current time
                let time = Utc::now();
                // Open the PrivateFile
                let file: &mut PrivateFile = root_dir
                    .open_file_mut(path_segments, true, time, metadata_forest, metadata, rng)
                    .await?;
                // Compress the data in the file on disk
                let file_content = compress_file(
                    &metadatas
                        .get(0)
                        .expect("why is there nothing in metadatas")
                        .canonicalized_path,
                )?;
                // Write the compressed bytes to the BlockStore / PrivateForest / PrivateDirectory
                file.set_content(time, file_content.as_slice(), content_forest, content, rng)
                    .await?;

                // Duplicates need to be linked no matter what
                for meta in &metadatas[1..] {
                    // Grab the original location
                    let dup = &meta.original_location;
                    let dup_path_segments = &path_to_segments(dup)?;
                    // Remove the final element to represent the folder path
                    let folder_segments = &dup_path_segments[..&dup_path_segments.len() - 1];
                    // Create that folder
                    root_dir
                        .mkdir(
                            folder_segments,
                            true,
                            Utc::now(),
                            metadata_forest,
                            metadata,
                            rng,
                        )
                        .await?;
                    // Copy the file from the original path to the duplicate path
                    root_dir
                        .cp_link(
                            path_segments,
                            dup_path_segments,
                            true,
                            metadata_forest,
                            metadata,
                        )
                        .await?;
                }
            }
            // If this is a directory or symlink
            PackPipelinePlan::Directory(meta) => {
                // Turn the canonicalized path into a vector of segments
                let path_segments = path_to_segments(&meta.original_location)?;

                // When path segments are empty we are unable to perform queries on the PrivateDirectory
                // Search through the PrivateDirectory for a Node that matches the path provided
                let result = root_dir
                    .get_node(&path_segments, true, metadata_forest, metadata)
                    .await;

                if let Ok(node) = result && node.is_some() {}
                // If there was an error searching for the Node or
                else {
                    // Create the subdirectory
                    root_dir
                        .mkdir(
                            &path_segments,
                            true,
                            Utc::now(),
                            metadata_forest,
                            metadata,
                            rng,
                        )
                        .await?;
                }
            }
            PackPipelinePlan::Symlink(_, _) => panic!("this is unreachable code"),
        }

        // Denote progress for each loop iteration
        progress_bar.inc(1);
    }

    // Now that the data exists, we can symlink to it
    for symlink_plan in symlink_plans {
        match symlink_plan {
            PackPipelinePlan::Symlink(meta, symlink_target) => {
                // The path where the symlink will be placed
                let symlink_segments = path_to_segments(&meta.original_location)?;

                // Link the file or folder
                root_dir
                    .write_symlink(
                        symlink_target
                            .to_str()
                            .expect("failed to represent as string")
                            .to_string(),
                        &symlink_segments,
                        true,
                        Utc::now(),
                        metadata_forest,
                        metadata,
                        rng,
                    )
                    .await?;
            }
            PackPipelinePlan::Directory(_) | PackPipelinePlan::FileGroup(_) => {
                panic!("this is unreachable code")
            }
        }

        // Denote progress for each loop iteration
        progress_bar.inc(1);
    }

    // Return Ok
    Ok(())
}
