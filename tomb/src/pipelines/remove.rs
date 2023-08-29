use super::error::PipelineError;
use crate::{types::config::globalconfig::GlobalConfig, utils::spider::path_to_segments};
use anyhow::Result;
use std::path::Path;

/// The pipeline for removing an individual file from a WNFS
pub async fn pipeline(origin: &Path, wnfs_path: &Path) -> Result<(), PipelineError> {
    // Global config
    let mut global = GlobalConfig::from_disk().await?;
    let wrapping_key = global.clone().wrapping_key().await?;
    // Bucket config
    if let Some(config) = global.get_bucket_by_origin(origin) {
        let (metadata_forest, content_forest, root_dir, manager) =
            &mut config.get_all(&wrapping_key).await?;
        // Attempt to remove the node
        root_dir
            .rm(
                &path_to_segments(wnfs_path)?,
                true,
                metadata_forest,
                &config.metadata,
            )
            .await?;

        // Store all the updated information, now that we've written the file
        config
            .set_all(metadata_forest, content_forest, root_dir, manager)
            .await?;

        // Update global
        global.update_config(&config)?;
        global.to_disk()?;
        Ok(())
    } else {
        Err(PipelineError::Uninitialized)
    }
}
