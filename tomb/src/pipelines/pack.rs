use crate::{
    types::spider::PackPipelinePlan,
    utils::{
        grouper::grouper,
        serialize::{
            load_key, load_manifest, store_all, store_key,
        },
        spider::{self, path_to_segments},
        wnfsio::{compress_file, get_progress_bar},
    },
};
use anyhow::Result;
use chrono::Utc;
use fs_extra::dir;
use indicatif::ProgressBar;
use log::info;
use rand::thread_rng;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
    vec,
};
use tomb_common::{types::{
    blockstore::{carblockstore::CarBlockStore, networkblockstore::NetworkBlockStore},
    pipeline::Manifest,
}, utils::serialize::{load_hot_forest, load_dir, load_cold_forest, store_dir}};
use wnfs::{
    common::BlockStore,
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateFile, PrivateForest},
};

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
    output_dir: Option<&Path>,
    _chunk_size: u64,
    follow_links: bool,
) -> Result<()> {
    // Create packing plan
    let packing_plan = create_plans(input_dir, follow_links).await?;
    // TODO: optionally turn off the progress bar
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = &get_progress_bar(packing_plan.len() as u64)?;
    // Path to the tomb folder
    let tomb_path = &input_dir.join(".tomb");
    // If initialization hasnt even happened
    if !tomb_path.exists() {
        panic!("Initialize this filesystem first with tomb init!");
    }
    // If the key has been made before
    let first_run = !tomb_path.join("root.key").exists();
    info!("pack first_run: {}", first_run);

    // If the user provided an output directory
    let local = output_dir.is_some();
    // Declare the MetaData store
    let mut hot_local = CarBlockStore::default();
    // Local content storage need only be valid if this is a local job
    let mut cold_local: CarBlockStore = if local {
        CarBlockStore::new(&output_dir.unwrap().join("content"), None)
    } else {
        CarBlockStore::default()
    };
    // Declare the remote stores
    let mut cold_remote = NetworkBlockStore::default();
    let mut hot_remote = NetworkBlockStore::default();

    // Load the manifest
    let manifest = load_manifest(tomb_path)?;
    // Update the BlockStores we will use if they have non-default values
    if manifest.hot_local != CarBlockStore::default() {
        hot_local = manifest.hot_local;
    }
    if manifest.hot_remote != NetworkBlockStore::default() {
        hot_remote = manifest.hot_remote;
    }
    if manifest.cold_local != CarBlockStore::default() {
        cold_local = manifest.cold_local;
    }
    if manifest.cold_remote != NetworkBlockStore::default() {
        cold_remote = manifest.cold_remote;
    }

    // Create the root directory in which all Nodes will be stored
    let mut root_dir = Rc::new(PrivateDirectory::new(
        Namefilter::default(),
        Utc::now(),
        &mut thread_rng(),
    ));
    // Create the PrivateForest from which Nodes will be queried
    let mut hot_forest = Rc::new(PrivateForest::new());
    let mut cold_forest = Rc::new(PrivateForest::new());

    // If this filesystem has never been packed
    if first_run {
        info!("tomb has not seen this filesystem before, starting from scratch! 💖");
        hot_local = CarBlockStore::new(tomb_path, None);
    }
    // If we've already packed this filesystem before
    else {
        println!("You've run tomb on this filesystem before! This may take some extra time, but don't worry, we're working hard to prevent duplicate work! 🔎");
        // Load the manifest
        let manifest = load_manifest(tomb_path)?;
        // Load in the Key
        let key = load_key(tomb_path, "root")?;

        // Load in the PrivateForest and PrivateDirectory
        if let Ok(new_hot_forest) = load_hot_forest(&manifest.roots, &manifest.hot_local).await &&
           let Ok(new_dir) = load_dir(&manifest, &key, &new_hot_forest, "current_root").await {
            // Update the forest and root directory
            hot_forest = new_hot_forest;
            root_dir = new_dir;
            println!("root dir and forest and original ratchet loaded from disk...");
        }
        // If the load was unsuccessful
        else {
            info!("Oh no! 😵 The metadata associated with this filesystem is corrupted, we have to pack from scratch.");
            hot_local = CarBlockStore::new(tomb_path, None);
        }

        let new_cold_forest = if local {
            load_cold_forest(&manifest.roots, &manifest.cold_local).await
        } else {
            load_cold_forest(&manifest.roots, &manifest.cold_remote).await
        };
        if let Ok(new_cold_forest) = new_cold_forest {
            cold_forest = new_cold_forest;
        }
    }

    // Process all of the PackPipelinePlans
    if local {
        // Process each of the packing plans with a local BlockStore
        process_plans(
            packing_plan,
            progress_bar,
            &mut root_dir,
            &mut hot_forest,
            &mut cold_forest,
            &hot_local,
            &cold_local,
        )
        .await?;
    } else {
        // Process each of the packing plans with a remote BlockStore
        process_plans(
            packing_plan,
            progress_bar,
            &mut root_dir,
            &mut hot_forest,
            &mut cold_forest,
            &hot_local,
            &cold_remote,
        )
        .await?;
    }

    // Construct the latest version of the Manifest struct
    let mut manifest = Manifest {
        version: env!("CARGO_PKG_VERSION").to_string(),
        cold_local,
        cold_remote,
        hot_local,
        hot_remote,
        roots: Default::default(),
    };

    if first_run {
        println!("storing original dir and key");
        let original_key =
            store_dir(&mut manifest, &mut hot_forest, &root_dir, "original_root").await?;
        store_key(tomb_path, &original_key, "original")?;
    }

    // Store Forest and Dir in BlockStores and retrieve Key
    let _ = store_all(
        local,
        tomb_path,
        &mut manifest,
        &mut hot_forest,
        &mut cold_forest,
        &root_dir,
    )
    .await?;

    if let Some(output_dir) = output_dir {
        // Remove the .tomb directory from the output path if it is already there
        let _ = std::fs::remove_dir_all(output_dir.join(".tomb"));
        // Copy the generated metadata into the output directory
        fs_extra::copy_items(
            &[tomb_path],
            output_dir,
            &dir::CopyOptions::new().overwrite(true),
        )
        .map_err(|e| anyhow::anyhow!("Failed to copy tomb dir: {}", e))?;
    }

    Ok(())
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
    hot_forest: &mut Rc<PrivateForest>,
    cold_forest: &mut Rc<PrivateForest>,
    hot_store: &impl BlockStore,
    cold_store: &impl BlockStore,
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
                // Open the PrivateFile
                let file: &mut PrivateFile = root_dir
                    .open_file_mut(path_segments, true, time, hot_forest, hot_store, rng)
                    .await?;
                // Compress the data in the file on disk
                let content = compress_file(
                    &metadatas
                        .get(0)
                        .expect("why is there nothing in metadatas")
                        .canonicalized_path,
                )?;
                // Write the compressed bytes to the BlockStore / PrivateForest / PrivateDirectory
                file.set_content(time, content.as_slice(), cold_forest, cold_store, rng)
                    .await?;

                // Duplicates need to be linked no matter what
                for metadata in &metadatas[1..] {
                    // Grab the original location
                    let dup = &metadata.original_location;
                    let dup_path_segments = &path_to_segments(dup)?;
                    // Remove the final element to represent the folder path
                    let folder_segments = &dup_path_segments[..&dup_path_segments.len() - 1];
                    // Create that folder
                    root_dir
                        .mkdir(
                            folder_segments,
                            true,
                            Utc::now(),
                            hot_forest,
                            hot_store,
                            rng,
                        )
                        .await?;
                    // Copy the file from the original path to the duplicate path
                    root_dir
                        .cp_link(
                            path_segments,
                            dup_path_segments,
                            true,
                            hot_forest,
                            hot_store,
                        )
                        .await?;
                }
            }
            // If this is a directory or symlink
            PackPipelinePlan::Directory(metadata) => {
                // Turn the canonicalized path into a vector of segments
                let path_segments = path_to_segments(&metadata.original_location)?;

                // When path segments are empty we are unable to perform queries on the PrivateDirectory
                // Search through the PrivateDirectory for a Node that matches the path provided
                let result = root_dir
                    .get_node(&path_segments, true, hot_forest, hot_store)
                    .await;

                // If there was an error searching for the Node or
                if result.is_err() || result.as_ref().unwrap().is_none() {
                    // Create the subdirectory
                    root_dir
                        .mkdir(&path_segments, true, Utc::now(), hot_forest, hot_store, rng)
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
            PackPipelinePlan::Symlink(metadata, symlink_target) => {
                // The path where the symlink will be placed
                let symlink_segments = path_to_segments(&metadata.original_location)?;

                // Link the file or folder
                root_dir
                    .write_symlink(
                        symlink_target.to_str().unwrap().to_string(),
                        &symlink_segments,
                        true,
                        Utc::now(),
                        hot_forest,
                        hot_store,
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
