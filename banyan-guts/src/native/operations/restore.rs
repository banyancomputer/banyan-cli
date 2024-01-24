use crate::{
    blockstore::{BanyanApiBlockStore, DoubleSplitStore, RootedBlockStore},
    filesystem::{wnfsio::path_to_segments, FsMetadata},
    native::{
        configuration::globalconfig::GlobalConfig, sync::OmniBucket, utils::get_progress_bar,
        NativeError,
    },
};
use std::{fs::File, io::Write, os::unix::fs::symlink, path::PathBuf};
use wnfs::private::PrivateNode;

/// Given the manifest file and a destination for our restored data, run the restoring pipeline
/// on the data referenced in the manifest.
///
/// # Arguments
///
/// * `fs` - FileSystem to modify
/// * `omni` - Context aware online / offline Drive
/// * `client` - Means of connecting to the server if need be
///
/// # Return Type
/// Returns `Ok(())` on success, otherwise returns an error.
pub async fn pipeline(mut omni: OmniBucket) -> Result<String, NativeError> {
    let fs = omni.unlock().await?;
    let local = omni.get_local()?;
    let mut global = GlobalConfig::from_disk().await?;
    let mut client = global.get_client().await?;
    // Announce that we're starting
    info!("🚀 Starting restoration pipeline...");
    let restored = omni.get_or_init_origin().await?;

    let metadata_store = &local.metadata;
    // Get all the nodes in the FileSystem
    let all_nodes = fs.get_all_nodes(metadata_store).await?;
    info!(
        "🔐 Restoring all {} files to {}",
        all_nodes.len(),
        restored.display()
    );

    if client.is_authenticated().await {
        let api_store = BanyanApiBlockStore::from(client.to_owned());
        let split_store = DoubleSplitStore::new(&local.content, &api_store);
        info!("Using online server as backup to grab file content...");
        restore_nodes(&fs, all_nodes, restored, metadata_store, &split_store).await?;
    } else {
        warn!("We notice you're offline or unauthenticated, reconstructing may fail if encrypted data is not already present on disk.");
        restore_nodes(&fs, all_nodes, restored, metadata_store, &local.content).await?;
    }

    global.update_config(&local)?;

    Ok("🎉 Data has been successfully reconstructed!".to_string())
}

/// Restore all nodes
pub async fn restore_nodes(
    fs: &FsMetadata,
    all_nodes: Vec<(PrivateNode, PathBuf)>,
    restored: PathBuf,
    metadata_store: &impl RootedBlockStore,
    content_store: &impl RootedBlockStore,
) -> Result<(), NativeError> {
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = get_progress_bar(all_nodes.len() as u64);
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
                    .await?;

                // If this file is a symlink
                if let Some(origin) = file.symlink_origin() {
                    // Write out the symlink
                    symlink(origin, built_path)?;
                } else {
                    // If the parent does not yet exist
                    if let Some(parent) = built_path.parent() {
                        if !parent.exists() {
                            // Create the directories
                            std::fs::create_dir_all(parent)?;
                        }
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
