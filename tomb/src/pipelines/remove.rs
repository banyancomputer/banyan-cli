use anyhow::Result;
use std::path::Path;

use crate::utils::{
    serialize::{load_pipeline, store_pipeline},
    spider::path_to_segments,
};

/// The pipeline for removing an individual file from a WNFS
pub async fn pipeline(tomb_path: &Path, wnfs_path: &Path) -> Result<()> {
    // Load everything from the metadata on disk
    let (_, manifest, forest, dir) = &mut load_pipeline(tomb_path).await?;
    // Attempt to remove the node
    dir.rm(
        &path_to_segments(wnfs_path)?,
        true,
        forest,
        &manifest.content_local,
    )
    .await?;
    // Stores the modified directory back to disk
    store_pipeline(tomb_path, manifest, forest, dir).await?;
    Ok(())
}
