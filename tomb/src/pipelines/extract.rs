use super::error::TombError;
use crate::{
    cli::command::BucketSpecifier, types::config::globalconfig::GlobalConfig,
    utils::extract::process_node,
};
use anyhow::Result;
use std::path::Path;

/// Given the manifest file and a destination for our extracted data, run the extracting pipeline
/// on the data referenced in the manifest.
///
/// # Arguments
///
/// * `output_dir` - &Path representing the relative path of the output directory in which to extract the data
/// * `manifest_file` - &Path representing the relative path of the manifest file
///
/// # Return Type
/// Returns `Ok(())` on success, otherwise returns an error.
pub async fn pipeline(
    global: &GlobalConfig,
    bucket_specifier: &BucketSpecifier,
    extracted: &Path,
) -> Result<String, TombError> {
    // Announce that we're starting
    info!("🚀 Starting extracting pipeline...");
    let wrapping_key = global.clone().wrapping_key().await?;
    let config = global.get_bucket_by_specifier(bucket_specifier)?;
    // Load metadata
    let (metadata_forest, content_forest, dir, _) = &mut config.get_all(&wrapping_key).await?;
    let metadata = &config.metadata;
    let content = &config.content;

    info!(
        "🔐 Decompressing and decrypting each file as it is copied to the new filesystem at {}",
        extracted.display()
    );

    // Run extraction on the base level with an empty built path
    process_node(
        metadata,
        content,
        metadata_forest,
        content_forest,
        &dir.as_node(),
        extracted,
        Path::new(""),
    )
    .await?;

    Ok(format!(
        "successfully extracted data into {}",
        extracted.display()
    ))
}
