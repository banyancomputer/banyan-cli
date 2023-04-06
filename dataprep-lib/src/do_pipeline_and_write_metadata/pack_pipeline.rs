use anyhow::{anyhow, Result};
use chrono::Utc;
use std::{
    collections::HashSet,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
    rc::Rc,
    vec,
};
use wnfs::{
    common::{AsyncSerialize, BlockStore, DiskBlockStore},
    namefilter::Namefilter,
    private::{PrivateDirectory, PrivateForest, PrivateRef},
};

use crate::{
    types::{
        pack_plan::PackPipelinePlan,
        unpack_plan::ManifestData, shared::CompressionScheme,
    },
    utils::{
        fs as fsutil,
        grouper::grouper,
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
    manifest_file: &Path,
    // TODO implement a way to specify chunk size for WNFS
    _chunk_size: u64,
    follow_links: bool,
) -> Result<()> {
    info!("🚀 Starting packing pipeline...");
    // Create the output directory
    fsutil::ensure_path_exists_and_is_empty_dir(output_dir, false)
        .expect("output directory must exist and be empty");

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
    let spidered_files =
        spider::spider(input_dir, follow_links, &mut seen_files).await?;
    packing_plan.extend(spidered_files);

    info!(
        "💾 Total number of files to pack: {}",
        packing_plan.len()
    );

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

    // Create a DiskBlockStore to store the packed data
    let store = DiskBlockStore::new(output_dir.to_path_buf());
    let mut rng = rand::thread_rng();
    let mut root_dir = Rc::new(PrivateDirectory::new(
        Namefilter::default(),
        Utc::now(),
        &mut rng,
    ));
    let mut forest = Rc::new(PrivateForest::new());

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

                // Write the compressed bytes to the BlockStore / PrivateForest / PrivateDirectory
                root_dir
                    .write(
                        &first_path_segments,
                        false,
                        Utc::now(),
                        compressed_bytes.clone(),
                        &mut forest,
                        &store,
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
                    root_dir.mkdir(folder_segments, false, Utc::now(), &forest, &store, &mut rng).await.unwrap();
                    // Copy the file from the original path to the duplicate path
                    root_dir.cp(
                        &first_path_segments,
                        &dup_path_segments,
                        false,
                        Utc::now(),
                        &mut forest,
                        &store,
                        &mut rng,
                    ).await.unwrap();
                }
            }
            // If this is a directory or symlink
            PackPipelinePlan::Directory(metadata) | PackPipelinePlan::Symlink(metadata, _) => {
                // Turn the canonicalized path into a vector of segments
                let path_segments = path_to_segments(&metadata.original_location).unwrap();
                // Create the subdirectory
                root_dir
                    .mkdir(&path_segments, false, Utc::now(), &forest, &store, &mut rng)
                    .await
                    .unwrap();
            }
        }

        // Denote progress for each loop iteration
        shared_pb.lock().unwrap().inc(1);
    }

    // Store the root of the PrivateDirectory in the BlockStore, retrieving a PrivateRef to it
    let root_ref: PrivateRef = root_dir.store(&mut forest, &store, &mut rng).await.unwrap();
    // Store it in the DiskBlockStore
    let ref_cid = store.put_serializable(&root_ref).await.unwrap();
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(&store).await.unwrap();
    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = store.put_serializable(&forest_ipld).await.unwrap();

    info!(
        "📄 Writing out a data manifest file to {}",
        manifest_file.display()
    );
    // For now just write out the content of compressed_and_encrypted to a file.
    // make sure the manifest file doesn't exist
    let manifest_writer = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(manifest_file)
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
        store,
        ref_cid,
        ipld_cid,
    };

    // Use serde to convert the ManifestData to JSON and write it to the path specified
    // Return the result of this operation
    serde_json::to_writer_pretty(manifest_writer, &manifest_data).map_err(|e| anyhow::anyhow!(e))
}
