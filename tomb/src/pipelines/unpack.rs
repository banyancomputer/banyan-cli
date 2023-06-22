use anyhow::Result;
use async_recursion::async_recursion;
use std::{path::Path, rc::Rc};
use tomb_common::types::config::globalconfig::GlobalConfig;
use wnfs::{
    common::BlockStore,
    private::{PrivateForest, PrivateNode},
};

use crate::{pipelines::error::PipelineError, utils::wnfsio::file_to_disk};

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
pub async fn pipeline(origin: &Path, output_dir: &Path) -> Result<()> {
    // Announce that we're starting
    info!("🚀 Starting unpacking pipeline...");

    let global = GlobalConfig::from_disk()?;

    if let Some(config) = global.get_bucket(origin) {
        // Load metadata
        let (metadata_forest, content_forest, dir) = &mut config.get_all().await?;
        let metadata = &config.metadata;
        let content = &config.content;

        info!(
            "🔐 Decompressing and decrypting each file as it is copied to the new filesystem at {}",
            output_dir.display()
        );

        #[async_recursion(?Send)]
        async fn process_node(
            output_dir: &Path,
            built_path: &Path,
            node: &PrivateNode,
            metadata_forest: &Rc<PrivateForest>,
            content_forest: &Rc<PrivateForest>,
            hot_store: &impl BlockStore,
            cold_store: &impl BlockStore,
        ) -> Result<()> {
            match &node {
                PrivateNode::Dir(dir) => {
                    println!("processing dir {}", built_path.display());
                    // Create the directory we are in
                    std::fs::create_dir_all(output_dir.join(built_path))?;
                    // Obtain a list of this Node's children
                    let node_names: Vec<String> = dir
                        .ls(&Vec::new(), true, metadata_forest, hot_store)
                        .await?
                        .into_iter()
                        .map(|(l, _)| l)
                        .collect();

                    // For each of those children
                    for node_name in node_names {
                        // Fetch the Node with the given name
                        if let Some(node) = dir
                            .get_node(&[node_name.clone()], true, metadata_forest, hot_store)
                            .await?
                        {
                            // Recurse with newly found node and await
                            process_node(
                                output_dir,
                                &built_path.join(node_name),
                                &node,
                                metadata_forest,
                                content_forest,
                                hot_store,
                                cold_store,
                            )
                            .await?;
                        }
                    }
                }
                PrivateNode::File(file) => {
                    // This is where the file will be unpacked no matter what
                    let file_path = &output_dir.join(built_path);
                    println!("processing file {}", file_path.display());
                    // Handle the PrivateFile and write its contents to disk
                    file_to_disk(
                        file,
                        output_dir,
                        file_path,
                        content_forest,
                        hot_store,
                        cold_store,
                    )
                    .await?;
                }
            }
            Ok(())
        }

        // TODO (organizedgrime) consult the WNFS gods as to why this is still necessary, considering we separated out our stores
        let total_forest = &Rc::new(content_forest.merge(metadata_forest, metadata).await?);

        // Run extraction on the base level with an empty built path
        process_node(
            output_dir,
            Path::new(""),
            &dir.as_node(),
            metadata_forest,
            total_forest,
            metadata,
            content,
        )
        .await
    } else {
        Err(PipelineError::Uninitialized().into())
    }
}
