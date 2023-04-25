use anyhow::Result;
use rand::thread_rng;
use std::{path::Path, rc::Rc};
use wnfs::{
    common::{AsyncSerialize, BlockStore, CarBlockStore},
    libipld::{serde as ipld_serde, Cid, Ipld},
    private::{PrivateDirectory, PrivateForest, PrivateNode, PrivateRef},
};

use crate::types::pipeline::ManifestData;

/// Deserializes the ManifestData struct from a given .meta dir
pub async fn load_manifest_data(input_meta_path: &Path) -> Result<ManifestData> {
    info!("Loading in cached metadata...");
    // The path in which we expect to find the Manifest JSON file
    let meta_file_path = input_meta_path.join("manifest.json");
    // Read in the manifest file from the metadata path
    let reader = std::fs::File::open(meta_file_path)
        .map_err(|e| anyhow::anyhow!("Failed to open manifest file: {}", e))?;

    // Deserialize the data read as the latest version of manifestdata
    let manifest_data: ManifestData = match serde_json::from_reader(reader) {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to deserialize manifest file: {}", e);
            panic!("Failed to deserialize manifest file: {e}");
        }
    };

    // If the major version of the manifest is not the same as the major version of the program
    if manifest_data.version.split('.').next().unwrap()
        != env!("CARGO_PKG_VERSION").split('.').next().unwrap()
    {
        // Panic if it's not
        error!("Unsupported manifest version.");
        panic!("Unsupported manifest version.");
    }

    Ok(manifest_data)
}

/// Loads in the PrivateForest and PrivateDirectory from a given ManifestData
pub async fn load_forest_and_dir(
    manifest_data: &ManifestData,
) -> Result<(Rc<PrivateForest>, Rc<PrivateDirectory>)> {
    info!("Loading in BlockStores and WNFS from metadata...");

    // Get the DiskBlockStores
    let content_store: &CarBlockStore = &manifest_data.content_store;
    let meta_store: &CarBlockStore = &manifest_data.meta_store;
    // Get all the root CIDs from metadata store
    let roots: Vec<Cid> = meta_store.get_roots();
    // Deserialize the PrivateRef
    let dir_ref: PrivateRef = meta_store
        .get_deserializable(&roots[0])
        .await
        .unwrap();
    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = meta_store
        .get_deserializable(&roots[1])
        .await
        .unwrap();

    // Create a PrivateForest from that IPLD DAG
    let forest: Rc<PrivateForest> =
        Rc::new(ipld_serde::from_ipld::<PrivateForest>(forest_ipld).unwrap());

    // Load the PrivateDirectory from the PrivateForest
    let dir: Rc<PrivateDirectory> = PrivateNode::load(&dir_ref, &forest, content_store)
        .await
        .unwrap()
        .as_dir()
        .unwrap();

    Ok((forest, dir))
}

/// Store the PrivateForest and PrivateDirectory in the content BlockStore
/// Return the CIDs of the references to those objects, which can be looked up in the Metadata BlockStore
pub async fn store_forest_and_dir(
    content_store: &mut CarBlockStore,
    meta_store: &mut CarBlockStore,
    forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<()> {
    // Random number generator
    let rng = &mut thread_rng();
    // Store the root of the PrivateDirectory in the BlockStore, retrieving a PrivateRef to it
    let root_ref: PrivateRef = root_dir.store(forest, content_store, rng).await?;
    // Determine the CID of the root directory, append it
    content_store.add_root(&root_ref.content_cid);
    // Store it in the Metadata CarBlockStore
    let ref_cid = meta_store.put_serializable(&root_ref).await?;
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(content_store).await?;
    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = meta_store.put_serializable(&forest_ipld).await?;
    // Add roots to meta store
    meta_store.add_root(&ref_cid);
    meta_store.add_root(&ipld_cid);
    // Return OK
    Ok(())
}
