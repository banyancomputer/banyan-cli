use crate::{
    types::unpack_plan::{ManifestData, UnpackPipelinePlan},
    vacuum::unpack::do_unpack_pipeline,
};
use anyhow::Result;
use std::path::Path;
use tokio_stream::StreamExt;

use indicatif::{ProgressBar, ProgressStyle};
use std::sync::{Arc, Mutex};

/// Given the manifest file and a destination for our unpacked data, run the unpacking pipeline
/// on the data referenced in the manifest.
///
/// # Arguments
///
/// * `output_dir` - &Path representing the relative path of the output directory in which to unpack the data
/// * `manifest_file` - &Path representing the relative path of the manifest file
///
/// # Return Type
/// Returns `Ok(())` on success, otherwise returns an error.
pub async fn unpack_pipeline(
    input_dir: &Path,
    output_dir: &Path,
    manifest_file: &Path,
) -> Result<()> {
    // parse manifest file into Vec<CodablePipeline>
    let reader = std::fs::File::open(manifest_file)
        .map_err(|e| anyhow::anyhow!("Failed to open manifest file: {}", e))?;

    info!("🚀 Starting unpacking pipeline...");

    // Deserialize the data read as the latest version of manifestdata
    let manifest_data: ManifestData = match serde_json::from_reader(reader) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to deserialize manifest file: {}", e);
            panic!("Failed to deserialize manifest file: {e}");
        }
    };

    // If the major version of the manifest is not the same as the major version of the program
    if manifest_data.version.split('.').next().unwrap()
        != env!("CARGO_PKG_VERSION").split('.').next().unwrap()
    {
        // Panic if it's not
        error!("Unsupported manifest version.");
        panic!("Unsupported manifest version.");
    }

    // Extract the unpacking plans
    let unpack_plans: Vec<UnpackPipelinePlan> = manifest_data.unpack_plans;

    info!(
        "🔐 Decompressing and decrypting each file as it is copied to the new filesystem at {}",
        output_dir.display()
    );
    let total_units = unpack_plans.iter().fold(0, |acc, x| acc + x.n_chunks()); // Total number of units of work to be processed
    // TODO buggy computation of n_chunks info!("🔧 Found {} file chunks, symlinks, and directories to unpack.", total_units);
    let total_size = unpack_plans.iter().fold(0, |acc, x| acc + x.n_bytes()); // Total number of bytes to be processed
    info!("💾 Total size of files to unpack: {}", byte_unit::Byte::from_bytes(total_size.into()).get_appropriate_unit(false).to_string());

    let pb = ProgressBar::new(total_units.try_into()?);
    pb.set_style(ProgressStyle::default_bar().template(
        "{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )?);
    let shared_pb = Arc::new(Mutex::new(pb));

    // Iterate over each pipeline
    tokio_stream::iter(unpack_plans)
        .then(|pipeline_to_disk| {
            do_unpack_pipeline(input_dir, pipeline_to_disk, output_dir, shared_pb.clone())
        })
        .collect::<Result<Vec<_>>>()
        .await?;

    // If the async block returns, we're Ok.
    Ok(())
}
