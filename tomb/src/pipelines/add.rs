use std::path::Path;

use super::error::PipelineError;
use crate::{
    types::config::globalconfig::GlobalConfig,
    utils::{spider::path_to_segments, wnfsio::compress_file},
};
use anyhow::Result;
use chrono::Utc;
use rand::thread_rng;

/// The pipeline for adding an individual file to a WNFS
pub async fn pipeline(
    origin: &Path,
    input_file: &Path,
    wnfs_path: &Path,
) -> Result<(), PipelineError> {
    // Global config
    let mut global = GlobalConfig::from_disk()?;
    let wrapping_key = global.wrapping_key_from_disk()?;

    // Bucket config
    if let Some(config) = global.get_bucket(origin) {
        // Get structs
        let (metadata_forest, content_forest, root_dir, key_manager) =
            &mut config.get_all(&wrapping_key).await?;

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
                &config.metadata,
                rng,
            )
            .await?;

        // Set file contents
        file.set_content(
            time,
            content_buf.as_slice(),
            content_forest,
            &config.content,
            rng,
        )
        .await?;

        // Store all the updated information, now that we've written the file
        config
            .set_all(
                &wrapping_key,
                metadata_forest,
                content_forest,
                root_dir,
                key_manager,
            )
            .await?;

        // Update global
        global.update_config(&config)?;
        global.to_disk()?;
        // Ok
        Ok(())
    } else {
        Err(PipelineError::Uninitialized)
    }
}
