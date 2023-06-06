use anyhow::Result;
use std::path::Path;

use crate::utils::{
    serialize::{load_all, store_all},
    spider::path_to_segments,
};

/// The pipeline for removing an individual file from a WNFS
pub async fn pipeline(tomb_path: &Path, wnfs_path: &Path) -> Result<()> {
    // Load everything from the metadata on disk
    let (_, manifest, forest, cold_forest, dir) = &mut load_all(true, tomb_path).await?;
    let path_segments = &path_to_segments(wnfs_path)?;
    // Attempt to remove the node
    dir.rm(path_segments, true, forest, &manifest.hot_local)
        .await?;
    // Stores the modified directory back to disk
    store_all(true, tomb_path, manifest, forest, cold_forest, dir).await?;
    Ok(())
}
