use crate::types::unpack_plan::ManifestData;
use anyhow::Result;
use std::path::Path;
use wnfs::libipld::Ipld;

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
    _input_dir: &Path,
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

    // Extract the IPLD DAG
    let _ipld: Ipld = manifest_data.ipld;

    info!(
        "🔐 Decompressing and decrypting each file as it is copied to the new filesystem at {}",
        output_dir.display()
    );

    //TODO (organizedgrime) - deserialize the IPLD DAG and implement the unpacking pipeline
    
    // If the async block returns, we're Ok.
    Ok(())
}
