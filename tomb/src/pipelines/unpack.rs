use anyhow::Result;
use async_recursion::async_recursion;
use std::{path::Path, rc::Rc};
use wnfs::{
    common::BlockStore,
    private::{PrivateForest, PrivateNode},
};

use super::error::PipelineError;
use crate::{types::config::globalconfig::GlobalConfig, utils::wnfsio::file_to_disk};

/// Given the manifest file and a destination for our unpacked data, run the unpacking pipeline
/// on the data referenced in the manifest.
///
/// # Arguments
///
/// * `output_dir` - &Path representing the relative path of the output directory in which to unpack the data
/// * `manifest_file` - &Path representing the relative path of the manifest file
///
/// # Return Type
/// Returns `Ok(())` on success, otherwise returns an error.
pub async fn pipeline(origin: &Path, unpacked: &Path) -> Result<(), PipelineError> {
    // Announce that we're starting
    info!("🚀 Starting unpacking pipeline...");

    let mut global = GlobalConfig::from_disk()?;
    let wrapping_key = global.wrapping_key_from_disk()?;

    if let Some(config) = global.get_bucket(origin) {
        // Load metadata
        let (metadata_forest, content_forest, dir, key_manager) =
            &mut config.get_all(&wrapping_key).await?;
        let metadata = &config.metadata;
        let content = &config.content;

        info!(
            "🔐 Decompressing and decrypting each file as it is copied to the new filesystem at {}",
            unpacked.display()
        );

        #[async_recursion(?Send)]
        async fn process_node(
            unpacked: &Path,
            built_path: &Path,
            node: &PrivateNode,
            metadata_forest: &Rc<PrivateForest>,
            content_forest: &Rc<PrivateForest>,
            metadata: &impl BlockStore,
            content: &impl BlockStore,
        ) -> Result<()> {
            match &node {
                PrivateNode::Dir(dir) => {
                    // Create the directory we are in
                    std::fs::create_dir_all(unpacked.join(built_path))?;
                    // Obtain a list of this Node's children
                    let node_names: Vec<String> = dir
                        .ls(&Vec::new(), true, metadata_forest, metadata)
                        .await?
                        .into_iter()
                        .map(|(l, _)| l)
                        .collect();

                    // For each of those children
                    for node_name in node_names {
                        // Fetch the Node with the given name
                        if let Some(node) = dir
                            .get_node(&[node_name.clone()], true, metadata_forest, metadata)
                            .await?
                        {
                            // Recurse with newly found node and await
                            process_node(
                                unpacked,
                                &built_path.join(node_name),
                                &node,
                                metadata_forest,
                                content_forest,
                                metadata,
                                content,
                            )
                            .await?;
                        }
                    }
                }
                PrivateNode::File(file) => {
                    // This is where the file will be unpacked no matter what
                    let file_path = &unpacked.join(built_path);
                    // Handle the PrivateFile and write its contents to disk
                    file_to_disk(file, unpacked, file_path, content_forest, content).await?;
                }
            }
            Ok(())
        }

        // Run extraction on the base level with an empty built path
        process_node(
            unpacked,
            Path::new(""),
            &dir.as_node(),
            metadata_forest,
            content_forest,
            metadata,
            content,
        )
        .await?;
        // Set all
        config
            .set_all(
                &wrapping_key,
                metadata_forest,
                content_forest,
                dir,
                key_manager,
            )
            .await?;
        global.update_config(&config)?;
        global.to_disk()?;
        Ok(())
    } else {
        Err(PipelineError::Uninitialized)
    }
}
