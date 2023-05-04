use crate::{
    types::{
        pipeline::{ManifestData, PackPipelinePlan},
        shared::CompressionScheme,
    },
    utils::{
        fs::{self as fsutil},
        grouper::grouper,
        pipeline::{load_forest_and_dir, load_manifest_data, store_forest_and_dir},
        spider::{self, path_to_segments},
    },
};
use anyhow::{anyhow, Result};
use blake2::{Blake2b512, Digest};
use chrono::Utc;
use fs_extra::dir;
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
    vec,
};
use wnfs::{
    common::CarBlockStore,
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
pub async fn pack_pipeline(
    input_dir: &Path,
    output_dir: &Path,
    // TODO implement a way to specify chunk size for WNFS
    _chunk_size: u64,
    follow_links: bool,
) -> Result<()> {
    info!("🚀 Starting packing pipeline...");
    // Create the output directory
    fsutil::ensure_path_exists_and_is_dir(output_dir).expect("output directory must exist");
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
    info!(
        "🔐 Compressing and encrypting each file as it is copied to the new filesystem at {}",
        output_dir.display()
    );

    // TODO: optionally turn off the progress bar
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = ProgressBar::new(packing_plan.len() as u64);
    // Stylize that progress bar!
    progress_bar.set_style(ProgressStyle::default_bar().template(
        "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )?);
    // Create a usable instance of the progress bar by wrapping the obj in Mutex and Arc
    let progress_bar = Arc::new(Mutex::new(progress_bar));

    // Create the directory in which content will be stored
    let content_path: PathBuf = output_dir.join("content");
    // Create a CarBlockStore
    let mut content_store: CarBlockStore = CarBlockStore::new(&content_path, None);
    // Create a random number generator
    let mut rng = rand::thread_rng();
    // Create the root directory in which all Nodes will be stored
    let mut root_dir = Rc::new(PrivateDirectory::new(
        Namefilter::default(),
        Utc::now(),
        &mut rng,
    ));
    // Create the PrivateForest from which Nodes will be queried
    let mut forest = Rc::new(PrivateForest::new());

    // This is the path in which we might find metadata from previous runs
    let input_meta_path = input_dir.join(".tomb");

    // Declare the MetaData store
    let mut meta_store: CarBlockStore;

    // If we've already packed this filesystem before
    if input_meta_path.exists() {
        println!("You've run tomb on this filesystem before! This may take some extra time, but don't worry, we're working hard to prevent duplicate work! 🔎");
        // Load in the ManifestData
        let manifest_data: ManifestData = load_manifest_data(&input_meta_path).await.unwrap();
        // Load in both CarBlockStores
        match load_forest_and_dir(&manifest_data).await {
            // If the load was successful
            Ok((new_forest, new_dir)) => {
                // Update the BlockStores
                meta_store = manifest_data.meta_store;
                content_store = manifest_data.content_store;
                // Update the forest and root directory
                forest = new_forest;
                root_dir = new_dir;
                println!("root dir and forest and original ratchet loaded from disk...");
            }
            // If the load was unsuccessful
            Err(_) => {
                info!("Oh no! 😵 The metadata associated with this filesystem is corrupted, we have to pack from scratch.");
                meta_store = CarBlockStore::new(&input_meta_path, None);
            }
        }
    }
    // If this filesystem has never been packed
    else {
        info!("tomb has not seen this filesystem before, starting from scratch! 💖");
        meta_store = CarBlockStore::new(&input_meta_path, None);
    }

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
                // Open the original file (just the first one!)
                let file = File::open(&metadatas.get(0)
                    .expect("why is there nothing in metadatas").canonicalized_path)
                    .map_err(|e| anyhow!("could not find canonicalized path when trying to open reader to original file! {}", e))?;

                // Create a reader for the original file
                let file_reader = BufReader::new(file);
                // Create a buffer to hold the compressed bytes
                let mut compressed_bytes: Vec<u8> = vec![];
                // Compress the chunk before feeding it to WNFS
                CompressionScheme::new_zstd()
                    .encode(file_reader, &mut compressed_bytes)
                    .unwrap();

                // Grab the metadata for the first occurrence of this file
                let first = &metadatas.get(0).unwrap().original_location;
                // Turn the relative path into a vector of segments
                let path_segments = path_to_segments(first).unwrap();
                // Grab the current time
                let time = Utc::now();

                // Search through the PrivateDirectory for a Node that matches the path provided
                let result = root_dir
                    .get_node(&path_segments, true, &forest, &content_store)
                    .await;

                // If the file does not exist in the PrivateForest or an error occurred in searching for it
                if result.is_err() || result.as_ref().unwrap().is_none() {
                    // Write the compressed bytes to the BlockStore / PrivateForest / PrivateDirectory
                    root_dir
                        .write(
                            &path_segments,
                            true,
                            time,
                            compressed_bytes.clone(),
                            &mut forest,
                            &content_store,
                            &mut rng,
                        )
                        .await
                        .unwrap();
                }
                // If the file exists in the PrivateForest
                else {
                    println!(
                        "a file at this path ({:?}) already exists in this forest",
                        &path_segments
                    );
                    // Forcibly cast because we know this is a file
                    let file: Rc<PrivateFile> = result.unwrap().unwrap().as_file().unwrap();
                    // Grab the content that already exists in the PrivateFile at this path
                    let existing_file_content = file.get_content(&forest, &content_store).await?;

                    // Create Hashers for both the new content and the old content
                    let mut h1 = Blake2b512::new();
                    let mut h2 = Blake2b512::new();
                    h1.update(&compressed_bytes);
                    h2.update(&existing_file_content);

                    // If the file has been modified since the last time it was packed
                    if h1.finalize() != h2.finalize() {
                        println!("The file at {:?} has changed between the previous packing and now, rewriting", path_segments);
                        // Write the new bytes to the path where the file was originally
                        // TODO (organizedgrime) - Here we need to do something with versioning!
                        root_dir
                            .write(
                                &path_segments,
                                true,
                                time,
                                compressed_bytes.clone(),
                                &mut forest,
                                &content_store,
                                &mut rng,
                            )
                            .await
                            .unwrap();
                    }
                    // TODO (organizedgrime) - actually check if the file is identical or a new version
                }

                // Duplicates need to be linked no matter what
                for metadata in &metadatas[1..] {
                    // Grab the original location
                    let dup = &metadata.original_location;
                    let dup_path_segments = path_to_segments(dup).unwrap();
                    // Remove the final element to represent the folder path
                    let folder_segments = &dup_path_segments[..&dup_path_segments.len() - 1];
                    // Create that folder
                    root_dir
                        .mkdir(
                            folder_segments,
                            true,
                            Utc::now(),
                            &forest,
                            &content_store,
                            &mut rng,
                        )
                        .await
                        .unwrap();
                    // Copy the file from the original path to the duplicate path
                    root_dir
                        .cp_link(
                            &path_segments,
                            &dup_path_segments,
                            true,
                            &mut forest,
                            &content_store,
                        )
                        .await
                        .unwrap();
                }
            }
            // If this is a directory or symlink
            PackPipelinePlan::Directory(metadata) => {
                // Turn the canonicalized path into a vector of segments
                let path_segments = path_to_segments(&metadata.original_location).unwrap();

                // When path segments are empty we are unable to perform queries on the PrivateDirectory
                // Search through the PrivateDirectory for a Node that matches the path provided
                let result = root_dir
                    .get_node(&path_segments, true, &forest, &content_store)
                    .await;

                // If there was an error searching for the Node or
                if result.is_err() || result.as_ref().unwrap().is_none() {
                    // Create the subdirectory
                    root_dir
                        .mkdir(
                            &path_segments,
                            true,
                            Utc::now(),
                            &forest,
                            &content_store,
                            &mut rng,
                        )
                        .await
                        .unwrap();
                }
                // We don't need an else here, directories don't actually contain any data
            }
            PackPipelinePlan::Symlink(_, _) => todo!(),
        }

        // Denote progress for each loop iteration
        progress_bar.lock().unwrap().inc(1);
    }

    // Now that the data exists, we can symlink to it
    for symlink_plan in symlink_plans {
        match symlink_plan {
            PackPipelinePlan::Symlink(metadata, symlink_target) => {
                // The path where the symlink will be placed
                let symlink_segments = path_to_segments(&metadata.original_location).unwrap();

                // Link the file or folder
                root_dir
                    .write_symlink(
                        symlink_target.to_str().unwrap().to_string(),
                        &symlink_segments,
                        true,
                        Utc::now(),
                        &forest,
                        &content_store,
                        &mut rng,
                    )
                    .await
                    .unwrap();
            }
            PackPipelinePlan::Directory(_) | PackPipelinePlan::FileGroup(_) => todo!(),
        }

        // Denote progress for each loop iteration
        progress_bar.lock().unwrap().inc(1);
    }

    // Construct output path for manifest data
    let manifest_file = input_meta_path.join("manifest.json");
    // For now just write out the content of compressed_and_encrypted to a file.
    // make sure the manifest file doesn't exist
    let manifest_writer = match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&manifest_file)
    {
        Ok(f) => f,
        Err(e) => {
            error!(
                "Failed to create manifest file at {}: {}",
                manifest_file.display(),
                e
            );
            Err(anyhow::anyhow!(
                "Failed to create manifest file at {}: {}",
                manifest_file.display(),
                e
            ))
            .unwrap()
        }
    };

    // Store Forest and Dir in BlockStores and retrieve CIDs
    store_forest_and_dir(&mut content_store, &mut meta_store, &mut forest, &root_dir)
        .await
        .unwrap();

    // Construct the latest version of the ManifestData struct
    let manifest_data = ManifestData {
        version: env!("CARGO_PKG_VERSION").to_string(),
        content_store,
        meta_store,
    };

    info!(
        "📄 Writing out a data manifest file to {}",
        manifest_file.display()
    );

    // Use serde to convert the ManifestData to JSON and write it to the path specified
    serde_json::to_writer_pretty(manifest_writer, &manifest_data)
        .map_err(|e| anyhow::anyhow!(e))?;

    // Remove the .tomb directory from the output path if it is already there
    let _ = fs::remove_dir_all(output_dir.join(".tomb"));
    // Copy the generated metadata into the output directory
    fs_extra::copy_items(
        &[input_meta_path],
        output_dir,
        &dir::CopyOptions::new().overwrite(true),
    )
    .map_err(|e| anyhow::anyhow!("Failed to copy meta dir: {}", e))?;

    // If we made it this far, all OK!
    Ok(())
}
