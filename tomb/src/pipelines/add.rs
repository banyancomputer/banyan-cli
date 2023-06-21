use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use rand::thread_rng;
use tomb_common::types::config::globalconfig::GlobalConfig;

use crate::utils::{spider::path_to_segments, wnfsio::compress_file};

use super::error::PipelineError;

/// The pipeline for adding an individual file to a WNFS
pub async fn pipeline(origin: &Path, input_file: &Path, wnfs_path: &Path) -> Result<()> {
    // Global config
    let mut global = GlobalConfig::from_disk()?;

    // Bucket config
    if let Some(config) = global.get_bucket(origin) {
        // Get structs
        let (_, metadata_forest, content_forest, root_dir) = &mut config.get_all().await?;

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
            .set_all(metadata_forest, content_forest, root_dir)
            .await?;

        // Update global
        global.update_config(&config)?;
        global.to_disk()
    } else {
        Err(PipelineError::Uninitialized().into())
    }
}
