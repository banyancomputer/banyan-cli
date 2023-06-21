use anyhow::Result;
use std::path::Path;
use tomb_common::types::config::globalconfig::GlobalConfig;

use crate::utils::spider::path_to_segments;

use super::error::PipelineError;

/// The pipeline for removing an individual file from a WNFS
pub async fn pipeline(origin: &Path, wnfs_path: &Path) -> Result<()> {
    // Global config
    let mut global = GlobalConfig::from_disk()?;
    // Bucket config
    if let Some(config) = global.get_bucket(origin) {
        let (_, metadata_forest, content_forest, dir) = &mut config.get_all().await?;
        // Attempt to remove the node
        dir.rm(
            &path_to_segments(wnfs_path)?,
            true,
            metadata_forest,
            &config.metadata,
        )
        .await?;

        // Stores the modified directory back to disk
        config.set_all(metadata_forest, content_forest, dir).await?;

        // Update global
        global.update_config(&config)?;
        global.to_disk()
    } else {
        Err(PipelineError::Uninitialized().into())
    }
}
