use crate::utils::{serialize::load_all, wnfsio::file_to_disk};
use anyhow::Result;
use async_recursion::async_recursion;
use fs_extra::{copy_items, dir::CopyOptions};
use std::path::Path;
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
pub async fn pipeline(input_dir: Option<&Path>, output_dir: &Path) -> Result<()> {
    // If there is an input dir specific with a valid tomb
    if let Some(input_dir) = input_dir && let tomb_dir = input_dir.join(".tomb") && tomb_dir.exists() {
        // Copy the existing tomb over to the output dir
        copy_items(&[tomb_dir], output_dir, &CopyOptions::new().overwrite(true))?;
    }

    // Paths representing metadata and content
    let tomb_path = output_dir.join(".tomb");
    // If initialization hasnt even happened
    if !tomb_path.exists() {
        panic!(".tomb does not exist in input or output directories");
    }
    // Announce that we're starting
    info!("🚀 Starting unpacking pipeline...");
    // If this is a local unpack
    let local = input_dir.is_some();
    // Load metadata
    let (_, mut manifest, forest, cold_forest, dir) = load_all(local, &tomb_path).await?;

    // Update the locations of the CarBlockStores to be relative to the input path
    // manifest.hot_local.change_dir(&tomb_path)?;
    // if local {
    //     manifest
    //         .cold_local
    //         .change_dir(&input_dir.unwrap().join("content"))?
    // }

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
                    .ls(&Vec::new(), true, forest, hot_store)
                    .await?
                    .into_iter()
                    .map(|(l, _)| l)
                    .collect();

                // For each of those children
                for node_name in node_names {
                    // Fetch the Node with the given name
                    if let Some(node) = dir
                        .get_node(&[node_name.clone()], true, forest, hot_store)
                        .await?
                    {
                        // Recurse with newly found node and await
                        process_node(
                            output_dir,
                            &built_path.join(node_name),
                            &node,
                            forest,
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
                file_to_disk(file, output_dir, file_path, forest, cold_store).await?;
            }
        }
        Ok(())
    }

    if local {
        // Run extraction on the base level with an empty built path
        process_node(
            output_dir,
            Path::new(""),
            &dir.as_node(),
            &forest,
            &manifest.hot_local,
            &manifest.cold_local,
        )
        .await
    } else {
        // Run extraction on the base level with an empty built path
        process_node(
            output_dir,
            Path::new(""),
            &dir.as_node(),
            &forest,
            &manifest.hot_local,
            &manifest.cold_remote,
        )
        .await
    }
}
