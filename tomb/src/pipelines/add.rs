use std::path::Path;

use super::error::TombError;
use crate::{cli::command::BucketSpecifier, types::config::globalconfig::GlobalConfig};
use anyhow::Result;
use chrono::Utc;
use rand::thread_rng;
use tomb_common::utils::wnfsio::{compress_file, path_to_segments};

/// The pipeline for adding an individual file to a WNFS
pub async fn pipeline(
    bucket_specifier: &BucketSpecifier,
    input_file: &Path,
    wnfs_path: &Path,
) -> Result<String, TombError> {
    // Global config
    let mut global = GlobalConfig::from_disk().await?;
    let wrapping_key = global.clone().wrapping_key().await?;

    // Bucket config
    let config = global.get_bucket_by_specifier(bucket_specifier)?;
    // Get structs
    let mut fs = config.unlock_fs(&wrapping_key).await?;

    // Compress the data in the file
    let content_buf = compress_file(input_file)?;
    // Turn the relative path into a vector of segments
    let time = Utc::now();
    let mut rng = thread_rng();
    let file = fs
        .root_dir
        .open_file_mut(
            &path_to_segments(wnfs_path)?,
            true,
            time,
            &mut fs.forest,
            &config.metadata,
            &mut rng,
        )
        .await?;

    // Set file contents
    file.set_content(
        time,
        content_buf.as_slice(),
        &mut fs.forest,
        &config.content,
        &mut rng,
    )
    .await?;

    // Store all the updated information, now that we've written the file
    config.save_fs(&mut fs).await?;

    // Update global
    global.update_config(&config)?;
    global.to_disk()?;
    // Ok
    Ok(format!(
        "successfully added {} to bucket",
        input_file.display()
    ))
}
