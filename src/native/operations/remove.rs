use crate::{
    filesystem::wnfsio::path_to_segments,
    native::{configuration::globalconfig::GlobalConfig, sync::LocalBucket, NativeError},
};
use std::path::Path;

/// The pipeline for removing an individual file from a WNFS
pub async fn pipeline(mut local: LocalBucket, wnfs_path: &Path) -> Result<(), NativeError> {
    // Global config
    let mut global = GlobalConfig::from_disk().await?;
    let wrapping_key = global.clone().wrapping_key().await?;

    let mut fs = local.unlock_fs(&wrapping_key).await?;
    // Attempt to remove the node
    fs.root_dir
        .rm(
            &path_to_segments(wnfs_path)?,
            true,
            &fs.forest,
            &local.metadata,
        )
        .await
        .map_err(Box::from)?;

    // Store all the updated information, now that we've written the file
    local.save_fs(&mut fs).await?;

    // Update global
    global.update_config(&local)?;
    Ok(())
}
