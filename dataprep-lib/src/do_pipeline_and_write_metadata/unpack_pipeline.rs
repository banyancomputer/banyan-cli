use crate::types::{pipeline::ManifestData, shared::CompressionScheme};
use anyhow::Result;
use async_recursion::async_recursion;
// use serde::{Deserialize, Serializer};
use std::{fs::File, io::Write, path::Path, rc::Rc};
use tokio as _;
use wnfs::{
    common::{BlockStore, CarBlockStore},
    libipld::{serde as ipld_serde, Ipld},
    private::{PrivateDirectory, PrivateForest, PrivateNode, PrivateRef},
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
pub async fn unpack_pipeline(
    _input_dir: &Path,
    output_dir: &Path,
    manifest_file: &Path,
) -> Result<()> {
    // parse manifest file into Vec<CodablePipeline>
    let reader = std::fs::File::open(manifest_file)
        .map_err(|e| anyhow::anyhow!("Failed to open manifest file: {}", e))?;

    info!("🚀 Starting unpacking pipeline...");

    // Deserialize the data read as the latest version of manifestdata
    let manifest_data: ManifestData = match serde_json::from_reader(reader) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to deserialize manifest file: {}", e);
            panic!("Failed to deserialize manifest file: {e}");
        }
    };

    // If the user specified a different location for their CarBlockStore
    // manifest_data.content_store.path = input_dir.to_path_buf();

    // If the major version of the manifest is not the same as the major version of the program
    if manifest_data.version.split('.').next().unwrap()
        != env!("CARGO_PKG_VERSION").split('.').next().unwrap()
    {
        // Panic if it's not
        error!("Unsupported manifest version.");
        panic!("Unsupported manifest version.");
    }

    // Get the DiskBlockStores
    let content_store: CarBlockStore = manifest_data.content_store;
    let meta_store: CarBlockStore = manifest_data.meta_store;

    // Deserialize the PrivateRef
    let dir_ref: PrivateRef = meta_store
        .get_deserializable(&manifest_data.ref_cid)
        .await
        .unwrap();

    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = meta_store
        .get_deserializable(&manifest_data.ipld_cid)
        .await
        .unwrap();

    // Create a PrivateForest from that IPLD DAG
    let forest: PrivateForest = ipld_serde::from_ipld::<_>(forest_ipld)?;

    // Load the PrivateDirectory from the PrivateForest
    let dir: Rc<PrivateDirectory> = PrivateNode::load(&dir_ref, &forest, &content_store)
        .await
        .unwrap()
        .as_dir()
        .unwrap();

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
        &content_store,
    )
    .await;

    //TODO (organizedgrime) - implement the unpacking pipeline
    Ok(())
}
