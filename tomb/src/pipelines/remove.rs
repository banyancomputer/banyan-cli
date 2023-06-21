use anyhow::Result;
use std::path::Path;
use tomb_common::utils::disk::*;

use crate::utils::spider::path_to_segments;

/// The pipeline for removing an individual file from a WNFS
pub async fn pipeline(tomb_path: &Path, wnfs_path: &Path) -> Result<()> {
    // Load everything from the metadata on disk
    let (_, config, metadata_forest, dir) = &mut hot_from_disk(tomb_path).await?;
    // Attempt to remove the node
    dir.rm(
        &path_to_segments(wnfs_path)?,
        true,
        metadata_forest,
        &config.metadata,
    )
    .await?;
    // Stores the modified directory back to disk
    hot_to_disk(config, metadata_forest, dir).await?;
    Ok(())
}
