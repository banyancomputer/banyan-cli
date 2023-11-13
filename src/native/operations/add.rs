use crate::{
    filesystem::wnfsio::{compress_file, path_to_segments},
    native::{
        configuration::{bucket::LocalBucket, globalconfig::GlobalConfig},
        operations::error::TombError,
    },
};
use chrono::Utc;
use rand::thread_rng;
use std::path::Path;

/// The pipeline for adding an individual file to a WNFS
pub async fn pipeline(
    local: LocalBucket,
    input_file: &Path,
    wnfs_path: &Path,
) -> Result<String, TombError> {
    // Global config
    let mut global = GlobalConfig::from_disk().await?;
    let wrapping_key = global.clone().wrapping_key().await?;
    // Get structs
    let mut fs = local.unlock_fs(&wrapping_key).await?;

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
            &local.metadata,
            &mut rng,
        )
        .await?;

    // Set file contents
    file.set_content(
        time,
        content_buf.as_slice(),
        &mut fs.forest,
        &local.content,
        &mut rng,
    )
    .await?;

    // Store all the updated information, now that we've written the file
    local.save_fs(&mut fs).await?;

    // Update global
    global.update_config(&local)?;
    global.to_disk()?;
    // Ok
    Ok(format!(
        "successfully added {} to bucket",
        input_file.display()
    ))
}
