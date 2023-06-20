use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use rand::thread_rng;
use tomb_common::{types::config::globalconfig::GlobalConfig, utils::disk::*};

use crate::utils::{spider::path_to_segments, wnfsio::compress_file};

/// The pipeline for adding an individual file to a WNFS
pub async fn pipeline(input_file: &Path, origin: &Path, wnfs_path: &Path) -> Result<()> {
    // Load the data
    let (_, metadata, content, metadata_forest, content_forest, root_dir) =
        &mut all_from_disk(origin).await?;
    // Compress the data in the file
    let content_buf = compress_file(input_file)?;
    // Turn the relative path into a vector of segments
    let time = Utc::now();
    let rng = &mut thread_rng();
    let file = root_dir
        .open_file_mut(
            &path_to_segments(wnfs_path)?,
            true,
            time,
            metadata_forest,
            metadata,
            rng,
        )
        .await?;
    //
    file.set_content(time, content_buf.as_slice(), content_forest, content, rng)
        .await?;

    // Store all the updated information, now that we've written the file
    all_to_disk(
        &GlobalConfig::get_bucket(&origin).unwrap(),
        metadata_forest,
        content_forest,
        root_dir,
    )
    .await?;
    // Return Ok
    Ok(())
}
