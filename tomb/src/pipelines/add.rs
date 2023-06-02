use std::path::Path;

use anyhow::Result;
use rand::thread_rng;

use crate::utils::{
    serialize::{load_pipeline, store_pipeline},
    spider::path_to_segments,
    wnfsio::{compress_file, write_file},
};

/// The pipeline for adding an individual file to a WNFS
pub async fn pipeline(input_file: &Path, tomb_path: &Path, wnfs_path: &Path) -> Result<()> {
    // Compress the data in the file
    let content = compress_file(input_file)?;
    // Turn the relative path into a vector of segments
    let path_segments = &path_to_segments(wnfs_path).unwrap();
    // Load the data
    let (_, manifest, forest, root_dir) = &mut load_pipeline(tomb_path).await?;
    // Write the file
    write_file(
        path_segments,
        content,
        root_dir,
        forest,
        &manifest.content_store,
        &mut thread_rng(),
    )
    .await?;
    // Store all the updated information, now that we've written the file
    store_pipeline(tomb_path, manifest, forest, root_dir).await?;
    // Return Ok
    Ok(())
}
