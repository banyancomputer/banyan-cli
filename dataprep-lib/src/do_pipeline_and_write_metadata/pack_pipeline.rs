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
        pack_plan::{PackPipelinePlan, PackPlan},
        shared::{CompressionScheme, EncryptionScheme, PartitionScheme},
        unpack_plan::ManifestData,
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
    chunk_size: u64,
    follow_links: bool,
) -> Result<()> {
    info!("🚀 Starting packing pipeline...");
    // Create the output directory
    fsutil::ensure_path_exists_and_is_empty_dir(output_dir, false)
        .expect("output directory must exist and be empty");

    // This pack plan is used to construct FileGroup type PackPipelinePlans,
    // but is not unique to any individual file / FileGroup.
    // remember to set the size_in_bytes field before use
    // prevents us from having to make a ton of new encryption keys (slow!!)
    let default_pack_plan = PackPlan {
        compression: CompressionScheme::new_zstd(),
        partition: PartitionScheme { chunk_size },
        encryption: EncryptionScheme::new_age(),
        size_in_bytes: 0,
    };

    // HashSet to track files that have already been seen
    let mut seen_files: HashSet<PathBuf> = HashSet::new();

    // Vector holding all the PackPipelinePlans for packing
    let mut packing_plan: Vec<PackPipelinePlan> = vec![];

    /* Perform deduplication and plan how to copy the files */
    info!("🔍 Deduplicating the filesystem at {}", input_dir.display());
    let group_plans = grouper(input_dir, follow_links, &default_pack_plan, &mut seen_files)?;
    packing_plan.extend(group_plans);

    /* Spider all the files so we can figure out what directories and symlinks to handle */
    // TODO fix setting follow_links / do it right
    info!(
        "📁 Finding directories and symlinks to back up starting at {}",
        input_dir.display()
    );
    let spidered_files =
        spider::spider(input_dir, follow_links, &default_pack_plan, &mut seen_files).await?;
    packing_plan.extend(spidered_files);

    // Total number of units of work to be processed
    let total_units = packing_plan.iter().fold(0, |acc, x| acc + x.n_chunks());
    // TODO buggy computation of n_chunks info!("🔧 Found {} file chunks, symlinks, and directories to pack.", total_units);
    // Total number of bytes to be processed
    let total_size = packing_plan.iter().fold(0, |acc, x| acc + x.n_bytes());
    info!(
        "💾 Total size of files to pack: {}",
        byte_unit::Byte::from_bytes(total_size)
            .get_appropriate_unit(false)
            .to_string()
    );

    info!(
        "🔐 Compressing and encrypting each file as it is copied to the new filesystem at {}",
        output_dir.display()
    );
    // Initialize the progress bar
    // TODO: optionally turn off the progress bar
    // compute the total number of units of work to be processed
    let pb = ProgressBar::new(total_units.into());
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
            PackPipelinePlan::FileGroup(metadatas, pack_plan) => {
                let PackPlan {
                    compression,
                    partition: _,
                    encryption: _,
                    size_in_bytes: _,
                } = pack_plan;

                // Open the original file (just the first one!)
                let file = File::open(&metadatas.get(0)
                    .expect("why is there nothing in metadatas").canonicalized_path)
                    .map_err(|e| anyhow!("could not find canonicalized path when trying to open reader to original file! {}", e))?;
                // Create a reader for the original file
                let file_reader = BufReader::new(file);
                // Create a buffer to hold the compressed bytes
                let mut compressed_bytes: Vec<u8> = vec![];
                // Encode and compress the chunk
                compression
                    .encode(file_reader, &mut compressed_bytes)
                    .unwrap();
                
                // Turn the canonicalized path into a vector of segments
                let path_segments =
                    path_to_segments(metadatas.get(0).unwrap().canonicalized_path.clone()).unwrap();

                // Write the compressed bytes to the BlockStore / PrivateForest / PrivateDirectory
                root_dir
                    .write(
                        &path_segments,
                        false,
                        Utc::now(),
                        compressed_bytes,
                        &mut forest,
                        &store,
                        &mut rng,
                    )
                    .await
                    .unwrap();
            }
            // If this is a directory or symlink
            PackPipelinePlan::Directory(metadata) | PackPipelinePlan::Symlink(metadata, _) => {
                // Turn the canonicalized path into a vector of segments
                let path_segments = path_to_segments(metadata.canonicalized_path.clone()).unwrap();
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
