use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use rand::thread_rng;

use crate::utils::{
    disk::{all_from_disk, all_to_disk},
    spider::path_to_segments,
    wnfsio::compress_file,
};

/// The pipeline for adding an individual file to a WNFS
pub async fn pipeline(
    local: bool,
    input_file: &Path,
    tomb_path: &Path,
    wnfs_path: &Path,
) -> Result<()> {
    // Load the data
    let (_, manifest, metadata_forest, content_forest, root_dir) =
        &mut all_from_disk(tomb_path).await?;
    // Compress the data in the file
    let content = compress_file(input_file)?;
    // Turn the relative path into a vector of segments
    let time = Utc::now();
    let rng = &mut thread_rng();
    let file = root_dir
        .open_file_mut(
            &path_to_segments(wnfs_path)?,
            true,
            time,
            metadata_forest,
            &manifest.metadata,
            rng,
        )
        .await?;
    //
    file.set_content(
        time,
        content.as_slice(),
        content_forest,
        &manifest.content,
        rng,
    )
    .await?;

    // Store all the updated information, now that we've written the file
    all_to_disk(
        tomb_path,
        manifest,
        metadata_forest,
        content_forest,
        root_dir,
    )
    .await?;
    // Return Ok
    Ok(())
}
