use super::error::TombError;
use crate::{
    banyan_api::{client::Client, models::metadata::Metadata},
    banyan_blockstore::{BanyanApiBlockStore, DoubleSplitStore, RootedBlockStore},
    banyan_filesystem::{metadata::FsMetadata, wnfsio::path_to_segments},
    banyan_native::{
        configuration::{bucket::OmniBucket, globalconfig::GlobalConfig},
        file_scanning::{grouper, spider, spider_plans::PreparePipelinePlan},
        utils::get_progress_bar,
    },
};
use anyhow::Result;
use std::{
    collections::HashSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use wnfs::private::PrivateNode;
/// Given the input directory, the output directory, the manifest file, and other metadata,
/// prepare the input directory into the output directory and store a record of how this
/// operation was performed in the manifest file.
///
/// # Arguments
///
/// * `fs` - FileSystem to modify
/// * `omni` - Context aware online / offline Drive
/// * `client` - Means of connecting to the server if need be
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

/// Create PreparePipelinePlans from an origin dir
pub async fn create_plans(origin: &Path, follow_links: bool) -> Result<Vec<PreparePipelinePlan>> {
    // HashSet to track files that have already been seen
    let mut seen_files: HashSet<PathBuf> = HashSet::new();
    // Vector holding all the PreparePipelinePlans for bundling
    let mut bundling_plan: Vec<PreparePipelinePlan> = vec![];

    info!("🔍 Deduplicating the filesystem at {}", origin.display());
    // Group the filesystem provided to detect duplicates
    let group_plans = grouper(origin, follow_links, &mut seen_files)?;
    // Extend the bundling plan
    bundling_plan.extend(group_plans);

    // TODO fix setting follow_links / do it right
    info!(
        "📁 Finding directories and symlinks to back up starting at {}",
        origin.display()
    );

    // Spider the filesystem provided to include directories and symlinks
    let spidered_files = spider(origin, follow_links, &mut seen_files).await?;
    // Extend the bundling plan
    bundling_plan.extend(spidered_files);

    info!(
        "💾 Total number of files to prepare: {}",
        bundling_plan.len()
    );

    Ok(bundling_plan)
}

/// Given a set of PreparePipelinePlans and required structs, process each
pub async fn process_plans(
    fs: &mut FsMetadata,
    bundling_plan: Vec<PreparePipelinePlan>,
    metadata_store: &impl RootedBlockStore,
    content_store: &impl RootedBlockStore,
) -> Result<()> {
    // Initialize the progress bar using the number of Nodes to process
    let progress_bar = get_progress_bar(bundling_plan.len() as u64)?;
    // Create vectors of direct and indirect plans
    let mut direct_plans: Vec<PreparePipelinePlan> = Vec::new();
    let mut symlink_plans: Vec<PreparePipelinePlan> = Vec::new();

    // Sort the bundling plans into plans which correspond to real data and those which are symlinks
    for prepare_pipeline_plan in bundling_plan {
        match prepare_pipeline_plan.clone() {
            PreparePipelinePlan::FileGroup(_) | PreparePipelinePlan::Directory(_) => {
                direct_plans.push(prepare_pipeline_plan);
            }
            PreparePipelinePlan::Symlink(_, _) => {
                symlink_plans.push(prepare_pipeline_plan);
            }
        }
    }

    // First, write data which corresponds to real data
    for direct_plan in direct_plans {
        match direct_plan {
            PreparePipelinePlan::FileGroup(metadatas) => {
                // Grab the metadata for the first occurrence of this file
                let first = &metadatas
                    .first()
                    .expect("no metadatas present")
                    .original_location;
                // Turn the relative path into a vector of segments
                let path_segments = path_to_segments(first)?;
                // Load the file from disk
                let mut file =
                    File::open(&metadatas.first().expect("no paths").canonicalized_path)?;
                let mut content = <Vec<u8>>::new();
                file.read_to_end(&mut content)?;
                // Add the file contents
                fs.write(&path_segments, metadata_store, content_store, content)
                    .await?;

                // Duplicates need to be linked no matter what
                for meta in &metadatas[1..] {
                    // Grab the original location
                    let dup_path_segments = path_to_segments(&meta.original_location)?;
                    if fs
                        .get_node(&dup_path_segments, metadata_store)
                        .await?
                        .is_none()
                    {
                        // Copy
                        fs.cp(&path_segments, &dup_path_segments, metadata_store)
                            .await?;
                    }
                }
            }
            // If this is a directory or symlink
            PreparePipelinePlan::Directory(meta) => {
                // Turn the canonicalized path into a vector of segments
                let path_segments = path_to_segments(&meta.original_location)?;
                // If the directory does not exist
                if fs.get_node(&path_segments, metadata_store).await.is_err() {
                    // Create the subdirectory
                    fs.mkdir(&path_segments, metadata_store).await?;
                }
            }
            PreparePipelinePlan::Symlink(_, _) => panic!("this is unreachable code"),
        }

        // Denote progress for each loop iteration
        progress_bar.inc(1);
    }

    // Now that the data exists, we can symlink to it
    for symlink_plan in symlink_plans {
        match symlink_plan {
            PreparePipelinePlan::Symlink(meta, symlink_target) => {
                // The path where the symlink will be placed
                let symlink_segments = path_to_segments(&meta.original_location)?;
                // Symlink it
                fs.symlink(&symlink_target, &symlink_segments, metadata_store)
                    .await?;
            }
            PreparePipelinePlan::Directory(_) | PreparePipelinePlan::FileGroup(_) => {
                panic!("this is unreachable code")
            }
        }

        // Denote progress for each loop iteration
        progress_bar.inc(1);
    }

    // Return Ok
    Ok(())
}
