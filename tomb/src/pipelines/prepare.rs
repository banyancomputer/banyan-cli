use std::path::PathBuf;

use super::error::TombError;
use crate::types::config::bucket::OmniBucket;
use crate::types::config::globalconfig::GlobalConfig;
use crate::types::spider::PreparePipelinePlan;
use crate::utils::prepare::{create_plans, process_plans};
use anyhow::Result;
use tomb_common::banyan_api::blockstore::BanyanApiBlockStore;
use tomb_common::banyan_api::client::Client;
use tomb_common::banyan_api::models::metadata::Metadata;
use tomb_common::blockstore::split::DoubleSplitStore;
use tomb_common::blockstore::RootedBlockStore;
use tomb_common::metadata::FsMetadata;
use tomb_common::utils::wnfsio::path_to_segments;
use wnfs::private::PrivateNode;
/// Given the input directory, the output directory, the manifest file, and other metadata,
/// prepare the input directory into the output directory and store a record of how this
/// operation was performed in the manifest file.
///
/// # Arguments
///
/// * `input_dir` - &Path representing the relative path of the input directory to prepare.
/// * `output_dir` - &Path representing the relative path of where to store the prepared data.
/// * `manifest_file` - &Path representing the relative path of where to store the manifest file.
/// * `chunk_size` - The maximum size of a prepared file / chunk in bytes.
/// * `follow_links` - Whether or not to follow symlinks when bundling.
///
/// # Return Type
/// Returns `Ok(())` on success, otherwise returns an error.
pub async fn pipeline(
    mut fs: FsMetadata,
    omni: &mut OmniBucket,
    client: &mut Client,
    follow_links: bool,
) -> Result<String, TombError> {
    // Local is non-optional
    let mut local = omni.get_local()?;

    // If there is a remote Bucket with metadatas that include a content root cid which has already been persisted
    if client.is_authenticated().await {
        if let Ok(remote) = omni.get_remote() {
            if let Ok(metadatas) = Metadata::read_all(remote.id, client).await {
                if metadatas.iter().any(|metadata| {
                    Some(metadata.root_cid.clone())
                        == local.content.get_root().map(|cid| cid.to_string())
                }) {
                    info!("Starting a new delta...");
                    local.content.add_delta()?;
                    omni.set_local(local.clone());
                }
            }
        }
    }

    // Create bundling plan
    let bundling_plan = create_plans(&local.origin, follow_links).await?;

    // Get all the paths present on disk
    let mut all_disk_paths = <Vec<PathBuf>>::new();
    for plan in bundling_plan.clone() {
        match plan {
            PreparePipelinePlan::Directory(metadata)
            | PreparePipelinePlan::Symlink(metadata, _) => {
                all_disk_paths.push(metadata.original_location.clone());
            }
            PreparePipelinePlan::FileGroup(metadatas) => {
                let paths: Vec<PathBuf> = metadatas
                    .iter()
                    .map(|metadata| metadata.original_location.clone())
                    .collect();
                all_disk_paths.extend(paths);
            }
        }
    }

    // Get all nodes and their associated paths
    let all_node_paths = fs.get_all_nodes(&local.metadata).await?;

    // Track all blocks removed since the last preparation
    for (node, wnfs_path) in all_node_paths {
        // If the existing WNFS node is not still represented on disk
        if !all_disk_paths.contains(&wnfs_path) {
            // If the node is a File, add all the CIDs associated with it to a list
            if let PrivateNode::File(file) = node {
                local
                    .deleted_block_cids
                    .extend(file.get_cids(&fs.forest, &local.metadata).await?);
            }
            // Remove the reference from the WNFS
            fs.rm(&path_to_segments(&wnfs_path)?, &local.metadata)
                .await?;
        }
    }

    let split_store_local = DoubleSplitStore::new(&local.content, &local.metadata);

    // If we're online, let's also spin up a BanyanApiBlockStore for getting content
    if let Ok(client) = GlobalConfig::from_disk().await?.get_client().await {
        let banyan_api_blockstore = BanyanApiBlockStore::from(client);
        let split_store_remote = DoubleSplitStore::new(&split_store_local, &banyan_api_blockstore);
        info!("Using online server as backup to check for file differences...");
        process_plans(&mut fs, bundling_plan, &local.metadata, &split_store_remote).await?;
    } else {
        warn!("We notice you're offline or unauthenticated, preparing may fail to detect content changes and require repreparation of old files.");
        process_plans(&mut fs, bundling_plan, &local.metadata, &split_store_local).await?;
    }

    local.save_fs(&mut fs).await?;
    omni.set_local(local);

    Ok(format!(
        "Prepared data successfully; Encrypted in {}",
        omni.get_local()?.content.path.display()
    ))
}
