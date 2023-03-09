use crate::{
    types::unpack_plan::{ManifestData, UnpackPipelinePlan},
    vacuum::unpack::do_unpack_pipeline,
};
use anyhow::Result;
use std::path::Path;
use tokio_stream::StreamExt;

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
pub async fn unpack_pipeline(output_dir: &Path, manifest_file: &Path) -> Result<()> {
    // parse manifest file into Vec<CodablePipeline>
    let reader = std::fs::File::open(manifest_file)?;

    // Deserialize the data read as the latest version of manifestdata
    let manifest_data: ManifestData = serde_json::from_reader(reader)?;

    // Check the version is what we want
    if manifest_data.version != env!("CARGO_PKG_VERSION") {
        // Panic if it's not
        panic!("Unsupported manifest version.");
    }

    // Extract the unpacking plans
    let unpack_plans: Vec<UnpackPipelinePlan> = manifest_data.unpack_plans;

    // Iterate over each pipeline
    tokio_stream::iter(unpack_plans)
        .then(|pipeline_to_disk| do_unpack_pipeline(pipeline_to_disk, output_dir))
        .collect::<Result<Vec<_>>>()
        .await?;

    // If the async block returns, we're Ok.
    Ok(())
}
