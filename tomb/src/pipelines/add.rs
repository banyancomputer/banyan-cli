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
    let (_, manifest, hot_forest, cold_forest, root_dir) =
        &mut all_from_disk(local, tomb_path).await?;
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
            hot_forest,
            &manifest.hot_local,
            rng,
        )
        .await?;
    //
    if local {
        file.set_content(
            time,
            content.as_slice(),
            cold_forest,
            &manifest.cold_local,
            rng,
        )
        .await?;
    } else {
        file.set_content(
            time,
            content.as_slice(),
            cold_forest,
            &manifest.cold_remote,
            rng,
        )
        .await?;
    }
    // Store all the updated information, now that we've written the file
    all_to_disk(
        local,
        tomb_path,
        manifest,
        hot_forest,
        cold_forest,
        root_dir,
    )
    .await?;
    // Return Ok
    Ok(())
}
