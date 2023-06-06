use anyhow::Result;
use std::path::Path;

use crate::utils::{
    serialize::{load_all_hot, store_all_hot},
    spider::path_to_segments,
};

/// The pipeline for removing an individual file from a WNFS
pub async fn pipeline(tomb_path: &Path, wnfs_path: &Path) -> Result<()> {
    // Load everything from the metadata on disk
    let (_, manifest, hot_forest, dir) = &mut load_all_hot(tomb_path).await?;
    let path_segments = &path_to_segments(wnfs_path)?;
    // Attempt to remove the node
    dir.rm(path_segments, true, hot_forest, &manifest.hot_local)
        .await?;
    // Stores the modified directory back to disk
    store_all_hot(tomb_path, manifest, hot_forest, dir).await?;
    Ok(())
}
