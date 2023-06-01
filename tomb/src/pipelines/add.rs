use std::path::Path;

use anyhow::Result;
use rand::thread_rng;

use crate::utils::{spider::path_to_segments, serialize::load_pipeline, write::{write_file, compress_file}};

/// 
pub async fn pipeline(input_file: &Path, tomb_path: &Path, wnfs_path: &Path) -> Result<()> {
    // Compress the data in the file
    let content = compress_file(input_file).await?;
    // Turn the relative path into a vector of segments
    let path_segments = &path_to_segments(wnfs_path).unwrap();
    // Load the data
    let (_, manifest, forest, dir) = &mut load_pipeline(tomb_path).await?;
    // Write the file
    write_file(path_segments, content, dir, forest, &manifest.content_store, &mut thread_rng()).await?;

    Ok(())
}
