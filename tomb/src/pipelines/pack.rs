use crate::{
    types::spider::PackPipelinePlan,
    utils::{
        grouper::grouper,
        spider::{self, path_to_segments},
        wnfsio::{compress_file, get_progress_bar},
    },
};
use anyhow::Result;
use chrono::Utc;
use indicatif::ProgressBar;
use log::info;
use rand::thread_rng;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
    vec,
};
use tomb_common::types::config::globalconfig::GlobalConfig;
use wnfs::{
    common::BlockStore,
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateFile, PrivateForest},
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
    input_dir: &Path,
    // _chunk_size: u64,
    follow_links: bool,
) -> Result<()> {
    // Create packing plan
    let packing_plan = create_plans(input_dir, follow_links).await?;
    // TODO: optionally turn off the progress bar
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = &get_progress_bar(packing_plan.len() as u64)?;

    let mut global = GlobalConfig::from_disk()?;

    // If the user has done initialization for this directory
    if let Some(config) = global.get_bucket(input_dir) {
        // println!("just loaded in BucketConfig from disk: {:?}", config);
        let metadata = &config.metadata;
        let content = &config.content;
        // Try to get the key of the root node
        let _ = config.get_key("root");

        // Create the root directory in which all Nodes will be stored
        let mut root_dir = Rc::new(PrivateDirectory::new(
            Namefilter::default(),
            Utc::now(),
            &mut thread_rng(),
        ));
        // Create the PrivateForest from which Nodes will be queried
        let mut metadata_forest = Rc::new(PrivateForest::new());
        let mut content_forest = Rc::new(PrivateForest::new());

        // If this filesystem has never been packed
        // if let Ok(key) = key {
        //     println!("You've run tomb on this filesystem before! This may take some extra time, but don't worry, we're working hard to prevent duplicate work! 🔎");
        //     let (new_metadata_forest, new_content_forest) = (
        //         load_metadata_forest(metadata).await,
        //         load_content_forest(content).await,
        //     );

        //     if let Ok(new_metadata_forest) = new_metadata_forest &&
        //         let Ok(new_dir) = load_dir(metadata, &key, &new_metadata_forest).await {
        //         // Update the forest and root directory
        //         metadata_forest = new_metadata_forest;
        //         root_dir = new_dir;
        //         println!("root dir and forest and original ratchet loaded from disk...");
        //     }
        //     // If the load was unsuccessful
        //     else {
        //         info!("Oh no! 😵 The metadata associated with this filesystem is corrupted, we have to pack from scratch.");
        //     }

        //     if let Ok(new_content_forest) = new_content_forest {
        //         content_forest = new_content_forest;
        //     }
        // } else {
        //     info!("tomb has not seen this filesystem before, starting from scratch! 💖");
        // }

        // Process all of the PackPipelinePlans
        process_plans(
            packing_plan,
            progress_bar,
            &mut root_dir,
            &mut metadata_forest,
            &mut content_forest,
            metadata,
            content,
        )
        .await?;

        // if first_run {
        //     println!("storing original dir and key");
        //     let original_key = store_dir(
        //         &mut manifest,
        //         &mut metadata_forest,
        //         &root_dir,
        //     )
        //     .await?;
        //     key_to_disk(tomb_path, &original_key, "original")?;
        // }

        // Store Forest and Dir in BlockStores and Key
        config
            .set_all(&mut metadata_forest, &mut content_forest, &root_dir)
            .await?;

        global.update_config(&config)?;

        global.to_disk()
    } else {
        Err(PipelineError::Uninitialized().into())
    }
}

async fn create_plans(input_dir: &Path, follow_links: bool) -> Result<Vec<PackPipelinePlan>> {
    // HashSet to track files that have already been seen
    let mut seen_files: HashSet<PathBuf> = HashSet::new();
    // Vector holding all the PackPipelinePlans for packing
    let mut packing_plan: Vec<PackPipelinePlan> = vec![];

    info!("🔍 Deduplicating the filesystem at {}", input_dir.display());
    // Group the filesystem provided to detect duplicates
    let group_plans = grouper(input_dir, follow_links, &mut seen_files)?;
    // Extend the packing plan
    packing_plan.extend(group_plans);

    // TODO fix setting follow_links / do it right
    info!(
        "📁 Finding directories and symlinks to back up starting at {}",
        input_dir.display()
    );

    // Spider the filesystem provided to include directories and symlinks
    let spidered_files = spider::spider(input_dir, follow_links, &mut seen_files).await?;
    // Extend the packing plan
    packing_plan.extend(spidered_files);

    info!("💾 Total number of files to pack: {}", packing_plan.len());

    Ok(packing_plan)
}

async fn process_plans(
    packing_plan: Vec<PackPipelinePlan>,
    progress_bar: &ProgressBar,
    root_dir: &mut Rc<PrivateDirectory>,
    metadata_forest: &mut Rc<PrivateForest>,
    content_forest: &mut Rc<PrivateForest>,
    metadata: &impl BlockStore,
    content: &impl BlockStore,
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
                let first = &metadatas.get(0).unwrap().original_location;
                // Turn the relative path into a vector of segments
                let path_segments = &path_to_segments(first)?;
                // Grab the current time
                let time = Utc::now();
                println!("path_segments: {:?}", path_segments);
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

                // If there was an error searching for the Node or
                if result.is_err() || result.as_ref().unwrap().is_none() {
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
                // We don't need an else here, directories don't actually contain any data
            }
            PackPipelinePlan::Symlink(_, _) => todo!(),
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
                        symlink_target.to_str().unwrap().to_string(),
                        &symlink_segments,
                        true,
                        Utc::now(),
                        metadata_forest,
                        metadata,
                        rng,
                    )
                    .await?;
            }
            PackPipelinePlan::Directory(_) | PackPipelinePlan::FileGroup(_) => todo!(),
        }

        // Denote progress for each loop iteration
        progress_bar.inc(1);
    }

    // Return Ok
    Ok(())
}
