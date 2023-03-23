use anyhow::Result;
use futures::{stream, StreamExt, TryStreamExt};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::{
    types::{
        pack_plan::{PackPipelinePlan, PackPlan},
        shared::{CompressionScheme, EncryptionScheme, PartitionScheme},
        unpack_plan::{ManifestData, UnpackPipelinePlan},
    },
    utils::{fs as fsutil, grouper::grouper, spider},
    vacuum::{self},
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
        writeout: output_dir.to_path_buf(),
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

    let total_units = packing_plan.iter().fold(0, |acc, x| acc + x.n_chunks()); // Total number of units of work to be processed
    // TODO buggy computation of n_chunks info!("🔧 Found {} file chunks, symlinks, and directories to pack.", total_units);
    let total_size = packing_plan.iter().fold(0, |acc, x| acc + x.n_bytes()); // Total number of bytes to be processed
    info!("💾 Total size of files to pack: {}", total_size);

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

    // TODO (laudiacay): For now we are doing compression in place, per-file. Make this better.
    let unpack_plans: Vec<UnpackPipelinePlan> = stream::iter(packing_plan)
        .then(|copy_plan| vacuum::pack::do_pack_pipeline(copy_plan, shared_pb.clone()))
        .try_fold(
            Vec::new(),
            |mut acc: Vec<UnpackPipelinePlan>, item: Vec<UnpackPipelinePlan>| async move {
                acc.extend(item);
                Ok(acc)
            },
        )
        .await?;

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
        unpack_plans,
    };

    // Use serde to convert the ManifestData to JSON and write it to the path specified
    // Return the result of this operation
    serde_json::to_writer_pretty(manifest_writer, &manifest_data).map_err(|e| anyhow::anyhow!(e))
}
