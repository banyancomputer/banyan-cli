use super::error::TombError;
use crate::{
    types::config::{bucket::OmniBucket, globalconfig::GlobalConfig},
    utils::restore::restore_nodes,
};
use anyhow::Result;
use tomb_common::{
    banyan_api::{blockstore::BanyanApiBlockStore, client::Client},
    blockstore::split::DoubleSplitStore,
    metadata::FsMetadata,
};

/// Given the manifest file and a destination for our restored data, run the restoring pipeline
/// on the data referenced in the manifest.
///
/// # Arguments
///
/// * `output_dir` - &Path representing the relative path of the output directory in which to restore the data
/// * `manifest_file` - &Path representing the relative path of the manifest file
///
/// # Return Type
/// Returns `Ok(())` on success, otherwise returns an error.
pub async fn pipeline(
    fs: FsMetadata,
    omni: &mut OmniBucket,
    client: &mut Client,
) -> Result<String, TombError> {
    // Announce that we're starting
    info!("🚀 Starting restoration pipeline...");
    let restored = omni
        .get_or_init_origin(&mut GlobalConfig::from_disk().await?)
        .await?;
    // Having a local bucket is non-optional
    let local = omni.get_local()?;

    let metadata_store = &local.metadata;
    // Get all the nodes in the FileSystem
    let all_nodes = fs.get_all_nodes(metadata_store).await?;
    info!(
        "🔐 Restoring all {} files to {}",
        all_nodes.len(),
        restored.display()
    );

    if client.is_authenticated().await {
        let banyan_api_store = BanyanApiBlockStore::from(client.to_owned());
        let split_store = DoubleSplitStore::new(&local.content, &banyan_api_store);
        info!("Using online server as backup to grab file content...");
        restore_nodes(&fs, all_nodes, restored, metadata_store, &split_store).await?;
    } else {
        warn!("We notice you're offline or unauthenticated, reconstructing may fail if encrypted data is not already present on disk.");
        restore_nodes(&fs, all_nodes, restored, metadata_store, &local.content).await?;
    }

    Ok("🎉 Data has been successfully reconstructed!".to_string())
}