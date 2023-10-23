use crate::{
    types::spider::PreparePipelinePlan,
    utils::{grouper::grouper, spider},
};
use anyhow::Result;
use indicatif::ProgressBar;
use std::{
    collections::HashSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use tomb_common::{
    blockstore::RootedBlockStore, metadata::FsMetadata, utils::wnfsio::path_to_segments,
};

/// Create PreparePipelinePlans from an origin dir
pub async fn create_plans(origin: &Path, follow_links: bool) -> Result<Vec<PreparePipelinePlan>> {
    // HashSet to track files that have already been seen
    let mut seen_files: HashSet<PathBuf> = HashSet::new();
    // Vector holding all the PreparePipelinePlans for bundling
    let mut bundling_plan: Vec<PreparePipelinePlan> = vec![];

    info!("🔍 Deduplicating the filesystem at {}", origin.display());
    // Group the filesystem provided to detect duplicates
    let group_plans = grouper(origin, follow_links, &mut seen_files)?;
    // Extend the bundling plan
    bundling_plan.extend(group_plans);

    // TODO fix setting follow_links / do it right
    info!(
        "📁 Finding directories and symlinks to back up starting at {}",
        origin.display()
    );

    // Spider the filesystem provided to include directories and symlinks
    let spidered_files = spider::spider(origin, follow_links, &mut seen_files).await?;
    // Extend the bundling plan
    bundling_plan.extend(spidered_files);

    info!(
        "💾 Total number of files to prepare: {}",
        bundling_plan.len()
    );

    Ok(bundling_plan)
}

/// Given a set of PreparePipelinePlans and required structs, process each
pub async fn process_plans(
    fs: &mut FsMetadata,
    bundling_plan: Vec<PreparePipelinePlan>,
    metadata_store: &impl RootedBlockStore,
    content_store: &impl RootedBlockStore,
    progress_bar: &ProgressBar,
) -> Result<()> {
    // Create vectors of direct and indirect plans
    let mut direct_plans: Vec<PreparePipelinePlan> = Vec::new();
    let mut symlink_plans: Vec<PreparePipelinePlan> = Vec::new();

    // Sort the bundling plans into plans which correspond to real data and those which are symlinks
    for prepare_pipeline_plan in bundling_plan {
        match prepare_pipeline_plan.clone() {
            PreparePipelinePlan::FileGroup(_) | PreparePipelinePlan::Directory(_) => {
                direct_plans.push(prepare_pipeline_plan);
            }
            PreparePipelinePlan::Symlink(_, _) => {
                symlink_plans.push(prepare_pipeline_plan);
            }
        }
    }

    println!("processing directs...");
    // First, write data which corresponds to real data
    for direct_plan in direct_plans {
        println!("processing {:?}", direct_plan);
        match direct_plan {
            PreparePipelinePlan::FileGroup(metadatas) => {
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
                println!("calling fs_write...");
                // Add the file contents
                fs.write(&path_segments, metadata_store, content_store, content)
                    .await?;
                println!("succeeded fs_write...");

                // Duplicates need to be linked no matter what
                for meta in &metadatas[1..] {
                    // Grab the original location
                    let dup_path_segments = path_to_segments(&meta.original_location)?;
                    if fs
                        .get_node(&dup_path_segments, metadata_store)
                        .await?
                        .is_none()
                    {
                        println!("calling cp...");
                        // Copy
                        fs.cp(&path_segments, &dup_path_segments, metadata_store)
                            .await?;
                        println!("finished cp...");
                    }
                }
            }
            // If this is a directory or symlink
            PreparePipelinePlan::Directory(meta) => {
                // Turn the canonicalized path into a vector of segments
                let path_segments = path_to_segments(&meta.original_location)?;
                // If the directory does not exist
                if fs.get_node(&path_segments, metadata_store).await.is_err() {
                    // Create the subdirectory
                    fs.mkdir(&path_segments, metadata_store).await?;
                }
            }
            PreparePipelinePlan::Symlink(_, _) => panic!("this is unreachable code"),
        }

        // Denote progress for each loop iteration
        progress_bar.inc(1);
    }

    println!("processing directs...");
    // Now that the data exists, we can symlink to it
    for symlink_plan in symlink_plans {
        println!("processing: {:?}", symlink_plan);
        match symlink_plan {
            PreparePipelinePlan::Symlink(meta, symlink_target) => {
                // The path where the symlink will be placed
                let symlink_segments = path_to_segments(&meta.original_location)?;
                // Symlink it
                fs.symlink(&symlink_target, &symlink_segments, metadata_store)
                    .await?;
            }
            PreparePipelinePlan::Directory(_) | PreparePipelinePlan::FileGroup(_) => {
                panic!("this is unreachable code")
            }
        }

        // Denote progress for each loop iteration
        progress_bar.inc(1);
    }

    // Return Ok
    Ok(())
}
