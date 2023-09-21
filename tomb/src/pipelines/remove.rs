use super::error::TombError;
use crate::{cli::command::BucketSpecifier, types::config::globalconfig::GlobalConfig};
use anyhow::Result;
use std::path::Path;
use tomb_common::utils::wnfsio::path_to_segments;

/// The pipeline for removing an individual file from a WNFS
pub async fn pipeline(
    bucket_specifier: &BucketSpecifier,
    wnfs_path: &Path,
) -> Result<(), TombError> {
    // Global config
    let mut global = GlobalConfig::from_disk().await?;
    let wrapping_key = global.clone().wrapping_key().await?;
    // Bucket config
    let config = global.get_bucket_by_specifier(bucket_specifier)?;

    let fs = &mut config.unlock_fs(&wrapping_key).await?;
    // Attempt to remove the node
    fs.root_dir
        .rm(
            &path_to_segments(wnfs_path)?,
            true,
            &fs.forest,
            &config.metadata,
        )
        .await?;

    // Store all the updated information, now that we've written the file
    config.save_fs(fs).await?;

    // Update global
    global.update_config(&config)?;
    global.to_disk()?;
    Ok(())
}
