use anyhow::Result;
use std::path::Path;
use tomb_common::types::config::globalconfig::GlobalConfig;

use crate::utils::spider::path_to_segments;

use super::error::PipelineError;

/// The pipeline for removing an individual file from a WNFS
pub async fn pipeline(origin: &Path, wnfs_path: &Path) -> Result<(), PipelineError> {
    // Global config
    let mut global = GlobalConfig::from_disk()?;
    // Bucket config
    if let Some(config) = global.get_bucket(origin) {
        let (metadata_forest, content_forest, dir, key_manager) = &mut config.get_all().await?;
        // Attempt to remove the node
        dir.rm(
            &path_to_segments(wnfs_path)?,
            true,
            metadata_forest,
            &config.metadata,
        )
        .await?;

        // Stores the modified directory back to disk
        config
            .set_all(metadata_forest, content_forest, dir, key_manager)
            .await?;

        // Update global
        global.update_config(&config)?;
        global.to_disk()?;
        Ok(())
    } else {
        Err(PipelineError::Uninitialized)
    }
}
