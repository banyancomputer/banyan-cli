use super::wnfsio::get_progress_bar;
use crate::{
    banyan_blockstore::RootedBlockStore,
    banyan_cli::pipelines::error::TombError,
    banyan_common::{metadata::FsMetadata, utils::wnfsio::path_to_segments},
};
use std::{fs::File, io::Write, os::unix::fs::symlink, path::PathBuf};
use wnfs::private::PrivateNode;

/// Restore all nodes
pub async fn restore_nodes(
    fs: &FsMetadata,
    all_nodes: Vec<(PrivateNode, PathBuf)>,
    restored: PathBuf,
    metadata_store: &impl RootedBlockStore,
    content_store: &impl RootedBlockStore,
) -> Result<(), TombError> {
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = get_progress_bar(all_nodes.len() as u64)?;
    // For each node path tuple in the FS Metadata
    for (node, path) in all_nodes {
        match node {
            PrivateNode::Dir(_) => {
                // Create the directory
                std::fs::create_dir_all(restored.join(path))?;
                progress_bar.inc(1);
            }
            PrivateNode::File(file) => {
                let built_path = restored.join(path.clone());

                let content = fs
                    .read(&path_to_segments(&path)?, metadata_store, content_store)
                    .await
                    .map_err(|err| {
                        TombError::custom_error(&format!(
                            "file missing: path: {} & err: {err}",
                            path.display()
                        ))
                    })?;

                // If this file is a symlink
                if let Some(origin) = file.symlink_origin() {
                    // Write out the symlink
                    symlink(origin, built_path)?;
                } else {
                    // If the parent does not yet exist
                    if let Some(parent) = built_path.parent()
                        && !parent.exists()
                    {
                        // Create the directories
                        std::fs::create_dir_all(parent)?;
                    }
                    // Create the file at the desired location
                    let mut output_file = File::create(built_path)?;

                    // Write out the content to disk
                    output_file.write_all(&content)?;
                }

                progress_bar.inc(1);
            }
        }
    }
    Ok(())
}
