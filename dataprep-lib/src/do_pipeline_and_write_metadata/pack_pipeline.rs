use anyhow::{anyhow, Result};
use chrono::Utc;
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
    private::{PrivateDirectory, PrivateForest, PrivateNode, PrivateRef},
};

use crate::{
    types::{
        pipeline::{ManifestData, PackPipelinePlan},
        shared::CompressionScheme,
    },
    utils::{
        fs::{self as fsutil},
        grouper::grouper,
        pipeline::{load_forest_dir, load_manifest_data},
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

    /* Perform deduplication and plan how to copy the files */
    info!("🔍 Deduplicating the filesystem at {}", input_dir.display());
    let group_plans = grouper(input_dir, follow_links, &mut seen_files)?;
    packing_plan.extend(group_plans);

    /* Spider all the files so we can figure out what directories and symlinks to handle */
    // TODO fix setting follow_links / do it right
    info!(
        "📁 Finding directories and symlinks to back up starting at {}",
        input_dir.display()
    );
    let spidered_files = spider::spider(input_dir, follow_links, &mut seen_files).await?;
    packing_plan.extend(spidered_files);

    info!("💾 Total number of files to pack: {}", packing_plan.len());

    info!(
        "🔐 Compressing and encrypting each file as it is copied to the new filesystem at {}",
        output_dir.display()
    );
    // Initialize the progress bar
    // TODO: optionally turn off the progress bar
    // compute the total number of units of work to be processed
    let pb = ProgressBar::new(packing_plan.len() as u64);
    pb.set_style(ProgressStyle::default_bar().template(
        "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )?);
    let shared_pb = Arc::new(Mutex::new(pb));

    let content_path = output_dir.to_path_buf().join("content");
    let mut content_store: CarBlockStore = CarBlockStore::new(content_path.clone(), None);

    // Experimenting with different RNGs to determine what is preventing duplicate detection
    let mut rng = rand::thread_rng();
    // let seed: Vec<u8> = vec![0; 32];
    // let seed_bytes: [u8; 32] = seed.try_into().unwrap();
    // let mut rng = StdRng::from_seed(seed_bytes);

    let mut root_dir = Rc::new(PrivateDirectory::new(
        Namefilter::default(),
        Utc::now(),
        &mut rng,
    ));
    let mut forest = Rc::new(PrivateForest::new());

    let meta_path = input_dir.join(".meta");

    println!(
        "the meta path is {} which exists: {}",
        meta_path.display(),
        meta_path.exists()
    );

    // TODO actually read the data and parse it to prevent double packing
    let meta_store;

    // If we've already packed this directory before
    if meta_path.exists() {
        println!("This directory has already been packed before! We're scanning it for duplicates");
        let manifest_data = load_manifest_data(&meta_path).await.unwrap();

        // If both data stores exist
        if manifest_data.meta_store.exists()? && manifest_data.content_store.exists()? {
            println!("both data stores exist");
            // manifest_data.content_store.change_dir(content_path).unwrap();
            // manifest_data.meta_store.change_dir(meta_path.clone()).unwrap();
            // Load them in
            let (new_forest, new_dir) = load_forest_dir(&manifest_data).await.unwrap();

            // Update the stores
            meta_store = manifest_data.meta_store;
            content_store = manifest_data.content_store;

            // Update the forest and root directory
            forest = new_forest;
            root_dir = new_dir;

            println!("loaded the private forest and directory");

            // let ls = root_dir.ls(&vec!["".to_string()], false, &forest, &content_store).await.unwrap();
            // println!("root_dir contents after load: {:?}", ls);
        } else {
            println!("those datastores dont point anywhere that exists");
            meta_store = CarBlockStore::new(meta_path.clone(), None);
        }
    } else {
        println!("i've never seen this directory before");
        meta_store = CarBlockStore::new(meta_path.clone(), None);
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
                // Encode and compress the chunk
                CompressionScheme::new_zstd()
                    .encode(file_reader, &mut compressed_bytes)
                    .unwrap();

                // Grab the metadata for the first occurrence of this file
                let first = &metadatas.get(0).unwrap().original_location;
                // Turn the canonicalized path into a vector of segments
                let first_path_segments = path_to_segments(first).unwrap();
                let time = Utc::now();

                info!("checking if this node already exists");
                // root_dir.lookup_node(path_segment, search_latest, forest, store)
                let node_query: Option<PrivateNode> = root_dir
                    .get_node(&first_path_segments, true, &forest, &content_store)
                    .await
                    .unwrap();
                if node_query.is_none() {
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
                } else {
                    println!("node exists in store already, skipping");
                }
            }
            // If this is a directory or symlink
            PackPipelinePlan::Directory(metadata) | PackPipelinePlan::Symlink(metadata, _) => {
                // Turn the canonicalized path into a vector of segments
                let path_segments = path_to_segments(&metadata.original_location).unwrap();
                // println!("checking if dir already exists at {:?}", path_segments);
                if !path_segments.is_empty() {
                    info!("searching");
                    let result = root_dir
                        .get_node(&path_segments, false, &forest, &content_store)
                        .await
                        .unwrap();
                    info!("search completed");

                    if result.is_some() {
                        println!("this directory already exists");
                    } else {
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
                }
            }
        }

        // Denote progress for each loop iteration
        shared_pb.lock().unwrap().inc(1);
    }

    // let ls = root_dir.ls(&vec!["".to_string()], false, &forest, &content_store).await.unwrap();
    // info!("ls data: {:?}", ls);

    let manifest_file = meta_path.join("manifest.json");
    // Store the root of the PrivateDirectory in the BlockStore, retrieving a PrivateRef to it
    let root_ref: PrivateRef = root_dir
        .store(&mut forest, &content_store, &mut rng)
        .await
        .unwrap();
    // Store it in the Metadata CarBlockStore
    let ref_cid = meta_store.put_serializable(&root_ref).await.unwrap();
    // Crea2te an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(&content_store).await.unwrap();

    // let forest_file = meta_path.join("fores.ipld");
    // let ipld_writer = std::fs::OpenOptions::new()
    //     .write(true)
    //     .create_new(true)
    //     .open(&forest_file)
    //     .unwrap();

    // serde_json::to_writer_pretty(ipld_writer, &forest_ipld).unwrap();

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
    // Return the result of this operation
    serde_json::to_writer_pretty(manifest_writer, &manifest_data)
        .map_err(|e| anyhow::anyhow!(e))?;

    let output_meta_path = output_dir.join(".meta");
    if output_meta_path.exists() {
        // Remove the meta directory from the output path if it is already there
        fs::remove_dir_all(output_meta_path)?;
        println!("removed .meta from output dir");
    } else {
        println!("didnt find a .meta in output to remove");
    }

    // fs::create_dir_all(new_meta_path)
    let copy_options = fs_extra::dir::CopyOptions::new().overwrite(true);
    //
    fs_extra::copy_items(&[meta_path], output_dir, &copy_options)
        .map_err(|e| anyhow::anyhow!("Failed to copy meta dir: {}", e))?;

    // std::fs::rename(output_dir.join(".meta"), output_dir.join("meta"))?;

    Ok(())
}
