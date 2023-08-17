use anyhow::Result;
use async_recursion::async_recursion;
use std::{path::Path, rc::Rc};
use wnfs::{
    common::BlockStore as WnfsBlockStore,
    private::{PrivateForest, PrivateNode},
};

use crate::pipelines::error::PipelineError;

use super::wnfsio::file_to_disk;

#[async_recursion(?Send)]
/// Recursively reconstruct each file and directory from the WNFS to disk
pub async fn process_node(
    metadata: &impl WnfsBlockStore,
    content: &impl WnfsBlockStore,
    metadata_forest: &Rc<PrivateForest>,
    content_forest: &Rc<PrivateForest>,
    node: &PrivateNode,
    unpacked: &Path,
    built_path: &Path,
) -> Result<()> {
    match &node {
        PrivateNode::Dir(dir) => {
            // Create the directory we are in
            std::fs::create_dir_all(unpacked.join(built_path))?;
            // Obtain a list of this Node's children
            let node_names: Vec<&String> = dir.get_entries().collect();
            
            println!("node names: {:?}", node_names);

            // For each of those children
            for node_name in node_names {
                
                println!("trying to find the node by name {}", node_name);
                // Fetch the Node with the given name

                if let Ok(Some(node)) = dir.lookup_node(node_name, true, metadata_forest, metadata).await {
                    // Recurse with newly found node and await
                    process_node(
                        metadata,
                        content,
                        metadata_forest,
                        content_forest,
                        &node,
                        unpacked,
                        &built_path.join(node_name),
                    )
                    .await?;
                }
                else {
                    return Err(PipelineError::FileNotFound(node_name.to_string()).into());
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
