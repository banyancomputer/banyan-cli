use anyhow::{anyhow, Result};
use chrono::Utc;
use fs_extra::dir;
use std::{
    collections::HashSet,
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
    rc::Rc,
    vec,
};
use wnfs::{
    common::{AsyncSerialize, BlockStore, CarBlockStore},
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateFile, PrivateForest, PrivateRef},
};

use crate::{
    types::{
        pipeline::{ManifestData, PackPipelinePlan},
        shared::CompressionScheme,
    },
    utils::{
        fs::{self as fsutil},
        grouper::grouper,
        pipeline::{load_forest_and_dir, load_manifest_data},
        spider::{self, path_to_segments},
    },
};

use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use std::sync::{Arc, Mutex};

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
    let input_meta_path = input_dir.join(".meta");

    // Declare the MetaData store
    let meta_store: CarBlockStore;

    // If we've already packed this filesystem before
    if input_meta_path.exists() {
        info!("You've run dataprep on this filesystem before! This may take some extra time, but don't worry, we're working hard to prevent duplicate work! 🔎");
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
        info!("Dataprep has not seen this filesystem before, starting from scratch! 💖");
        meta_store = CarBlockStore::new(&input_meta_path, None);
    }

    // TODO (organizedgrime) async these for real...
    for pack_pipeline_plan in packing_plan {
        match pack_pipeline_plan {
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
                let first_path_segments = path_to_segments(first).unwrap();
                // Grab the current time
                let time = Utc::now();

                // Search through the PrivateDirectory for a Node that matches the path provided
                let result = root_dir
                    .get_node(&first_path_segments, true, &forest, &content_store)
                    .await;

                // If the file does not exist in the PrivateForest or an error occurred in searching for it
                if result.is_err() || result.as_ref().unwrap().is_none() {
                    // Write the compressed bytes to the BlockStore / PrivateForest / PrivateDirectory
                    root_dir
                        .write(
                            &first_path_segments,
                            false,
                            time,
                            compressed_bytes.clone(),
                            &mut forest,
                            &content_store,
                            &mut rng,
                        )
                        .await
                        .unwrap();

                    // For each duplicate
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
                                false,
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
                                &first_path_segments,
                                &dup_path_segments,
                                false,
                                &mut forest,
                                &content_store,
                            )
                            .await
                            .unwrap();
                    }
                }
                // If the file exists in the PrivateForest
                else {
                    // Forcibly cast because we know this is a file
                    let _file: Rc<PrivateFile> = result.unwrap().unwrap().as_file().unwrap();
                    // TODO (organizedgrime) - actually check if the file is identical or a new version
                }
            }
            // If this is a directory or symlink
            PackPipelinePlan::Directory(metadata) | PackPipelinePlan::Symlink(metadata, _) => {
                // Turn the canonicalized path into a vector of segments
                let path_segments = path_to_segments(&metadata.original_location).unwrap();

                // When path segments are empty we are unable to perform queries on the PrivateDirectory
                if !path_segments.is_empty() {
                    // Search through the PrivateDirectory for a Node that matches the path provided
                    let result = root_dir
                        .get_node(&path_segments, false, &forest, &content_store)
                        .await;

                    if result.is_err() || result.as_ref().unwrap().is_none() {
                        println!("None found!");
                        // println!("this directory doesnt already exist");
                        // Create the subdirectory
                        root_dir
                            .mkdir(
                                &path_segments,
                                false,
                                Utc::now(),
                                &forest,
                                &content_store,
                                &mut rng,
                            )
                            .await
                            .unwrap();
                    }
                    else {
                        // Forcibly cast because we know this is a dir
                        let _dir: Rc<PrivateDirectory> = result.unwrap().unwrap().as_dir().unwrap();
                        // TODO(organizedgrime): determine if this node has been modified and needs rewriting
                    }
                }
            }
        }

        // Denote progress for each loop iteration
        progress_bar.lock().unwrap().inc(1);
    }

    let manifest_file = input_meta_path.join("manifest.json");
    // Store the root of the PrivateDirectory in the BlockStore, retrieving a PrivateRef to it
    let root_ref: PrivateRef = root_dir
        .store(&mut forest, &content_store, &mut rng)
        .await
        .unwrap();
    // Store it in the Metadata CarBlockStore
    let ref_cid = meta_store.put_serializable(&root_ref).await.unwrap();
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(&content_store).await.unwrap();
    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = meta_store.put_serializable(&forest_ipld).await.unwrap();

    info!(
        "📄 Writing out a data manifest file to {}",
        manifest_file.display()
    );
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

    // Construct the latest version of the ManifestData struct
    let manifest_data = ManifestData {
        version: env!("CARGO_PKG_VERSION").to_string(),
        content_store,
        meta_store,
        ref_cid,
        ipld_cid,
    };

    // Use serde to convert the ManifestData to JSON and write it to the path specified
    serde_json::to_writer_pretty(manifest_writer, &manifest_data)
        .map_err(|e| anyhow::anyhow!(e))?;

    // Remove the .meta directory from the output path if it is already there
    let _ = fs::remove_dir_all(output_dir.join(".meta"));
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
