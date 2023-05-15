use crate::{
    types::shared::CompressionScheme,
    utils::pipeline::{load_dir, load_forest, load_manifest_and_key, load_pipeline},
};
use anyhow::Result;
use async_recursion::async_recursion;
use std::{fs::File, io::Write, path::Path};
use tokio::{self as _, fs::symlink};
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
    let tomb_path = input_dir.join(".tomb");

    // Announce that we're starting
    info!("🚀 Starting unpacking pipeline...");

    // Load
    let (_, manifest_data, forest, dir) = load_pipeline(&tomb_path).await?;

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
        match &node {
            PrivateNode::Dir(dir) => {
                // Create the directory we are in
                std::fs::create_dir_all(output_dir.join(built_path)).unwrap();
                // Obtain a list of this Node's children
                let node_names: Vec<String> = dir
                    .ls(&Vec::new(), true, forest, store)
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|(l, _)| l)
                    .collect();

                // For each of those children
                for node_name in node_names {
                    // Fetch the Node with the given name
                    let node = dir
                        .get_node(&[node_name.clone()], true, forest, store)
                        .await
                        .unwrap()
                        .unwrap();

                    // Recurse with newly found node and await
                    process_node(
                        output_dir,
                        &built_path.join(node_name),
                        &node,
                        forest,
                        store,
                    )
                    .await;
                }
            }
            PrivateNode::File(file) => {
                // This is where the file will be unpacked no matter what
                let file_path = output_dir.join(built_path);
                // If this file is a symlink
                if let Some(path) = file.symlink_origin() {
                    // Write out the symlink
                    symlink(output_dir.join(path), file_path).await.unwrap();
                }
                // If this is a real file
                else {
                    // Get the bytes associated with this file
                    let file_content = file.get_content(forest, store).await.unwrap();
                    // Create a buffer to hold the decompressed bytes
                    let mut decompressed_bytes: Vec<u8> = vec![];
                    // Decompress the chunk before writing to disk
                    CompressionScheme::new_zstd()
                        .decode(file_content.as_slice(), &mut decompressed_bytes)
                        .unwrap();
                    // Create the file at the desired location
                    let mut output_file = File::create(file_path).unwrap();
                    // Write all contents to the output file
                    output_file.write_all(&decompressed_bytes).unwrap();
                }
            }
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

    // Remove the .tomb directory from the output path if it is already there
    let _ = std::fs::remove_dir_all(output_dir.join(".tomb"));
    // Copy the cached metadata into the output directory
    fs_extra::copy_items(
        &[tomb_path],
        output_dir,
        &fs_extra::dir::CopyOptions::new(),
    )
    .map_err(|e| anyhow::anyhow!("Failed to copy meta dir: {}", e))?;

    Ok(())
}
