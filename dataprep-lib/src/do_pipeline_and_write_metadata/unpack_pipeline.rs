use crate::{
    types::shared::CompressionScheme,
    utils::pipeline::{load_forest_dir, load_manifest_data},
};
use anyhow::Result;
use async_recursion::async_recursion;
// use serde::{Deserialize, Serializer};
use std::{fs::File, io::Write, path::Path};
use tokio as _;
use wnfs::{
    common::BlockStore,
    private::{PrivateForest, PrivateNode},
};

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
pub async fn unpack_pipeline(input_dir: &Path, output_dir: &Path) -> Result<()> {
    // Paths representing metadata and content
    let meta_path = input_dir.join(".meta");
    let content_path = input_dir.join("content");

    // Announce that we're starting
    info!("🚀 Starting unpacking pipeline...");

    // Load in the Private Forest and the PrivateDirectory from the metadata directory
    let mut manifest_data = load_manifest_data(meta_path.as_path()).await.unwrap();
    // Update the directories
    manifest_data.content_store.change_dir(content_path)?;
    manifest_data.meta_store.change_dir(meta_path.clone())?;
    let (forest, dir) = load_forest_dir(&manifest_data).await.unwrap();

    info!(
        "🔐 Decompressing and decrypting each file as it is copied to the new filesystem at {}",
        output_dir.display()
    );

    #[async_recursion(?Send)]
    async fn process_node(
        output_dir: &Path,
        built_path: &Path,
        node: &PrivateNode,
        forest: &PrivateForest,
        store: &impl BlockStore,
    ) {
        // If we are processing a directory
        if node.is_dir() {
            // Create the directory we are in
            std::fs::create_dir_all(output_dir.join(built_path)).unwrap();

            let dir = node.as_dir().unwrap();
            // List
            let ls = dir.ls(&Vec::new(), false, forest, store).await.unwrap();
            let node_names: Vec<String> = ls.into_iter().map(|(l, _)| l).collect();

            for node_name in node_names {
                let paths = &vec![node_name.clone()];
                let node = dir
                    .get_node(paths, false, forest, store)
                    .await
                    .unwrap()
                    .unwrap();

                // Recurse with newly found node
                process_node(
                    output_dir,
                    built_path.join(node_name).as_path(),
                    &node,
                    forest,
                    store,
                )
                .await;
            }
        }
        // This implies node.is_file() == true
        else {
            let file = node.as_file().unwrap();
            // Get the bytes associated with this file
            let file_content = file.get_content(forest, store).await.unwrap();
            // Create a buffer to hold the decompressed bytes
            let mut decompressed_bytes: Vec<u8> = vec![];
            // Encode and compress the chunk
            CompressionScheme::new_zstd()
                .decode(file_content.as_slice(), &mut decompressed_bytes)
                .unwrap();
            // Create the file at this location
            let mut output_file = File::create(output_dir.join(built_path)).unwrap();
            // Write the contents to the output file
            output_file.write_all(&decompressed_bytes).unwrap();
        }
    }

    // Run extraction on the base level with an empty built path
    process_node(
        output_dir,
        Path::new(""),
        &dir.as_node(),
        &forest,
        &manifest_data.content_store,
    )
    .await;

    fs_extra::copy_items(&[meta_path], output_dir, &fs_extra::dir::CopyOptions::new())
        .map_err(|e| anyhow::anyhow!("Failed to copy meta dir: {}", e))?;

    //TODO (organizedgrime) - implement the unpacking pipeline
    Ok(())
}
