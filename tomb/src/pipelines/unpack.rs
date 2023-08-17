use super::error::PipelineError;
use crate::{types::config::globalconfig::GlobalConfig, utils::unpack::process_node};
use anyhow::Result;
use std::path::Path;

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
pub async fn pipeline(origin: &Path, unpacked: &Path) -> Result<(), PipelineError> {
    // Announce that we're starting
    info!("🚀 Starting unpacking pipeline...");
    
    let global = GlobalConfig::from_disk().await?;
    println!("obtained global config");
    let wrapping_key = global.load_key().await?;
    println!("obtained key");

    if let Some(config) = global.get_bucket(origin) {
        println!("obtained config");
        // Load metadata
        let (metadata_forest, content_forest, dir, _, _) =
            &mut config.get_all(&wrapping_key).await?;
        let metadata = &config.metadata;
        let content = &config.content;

        info!(
            "🔐 Decompressing and decrypting each file as it is copied to the new filesystem at {}",
            unpacked.display()
        );

        // Run extraction on the base level with an empty built path
        process_node(
            metadata,
            content,
            metadata_forest,
            content_forest,
            &dir.as_node(),
            unpacked,
            Path::new(""),
        )
        .await?;

        Ok(())
    } else {
        Err(PipelineError::Uninitialized)
    }
}
