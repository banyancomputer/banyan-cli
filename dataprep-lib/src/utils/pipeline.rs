use anyhow::Result;
use std::{path::Path, rc::Rc};
use wnfs::{
    common::{BlockStore, CarBlockStore},
    libipld::{serde as ipld_serde, Ipld},
    private::{PrivateDirectory, PrivateForest, PrivateNode, PrivateRef},
};

use crate::types::pipeline::ManifestData;

/// Deserializes the ManifestData struct from a given .meta dir
pub async fn load_manifest_data(input_meta_path: &Path) -> Result<ManifestData> {
    println!("loading manifest data in {}", input_meta_path.display());
    let meta_file_path = input_meta_path.join("manifest.json");
    println!(
        "attempting to open the file at {}, which exists: {}",
        meta_file_path.display(),
        meta_file_path.exists()
    );
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

    Ok(manifest_data)
}

/// Loads in the PrivateForest and PrivateDirectory from a given ManifestData
pub async fn load_forest_and_dir(
    manifest_data: &ManifestData,
) -> Result<(Rc<PrivateForest>, Rc<PrivateDirectory>)> {
    // If the major version of the manifest is not the same as the major version of the program
    if manifest_data.version.split('.').next().unwrap()
        != env!("CARGO_PKG_VERSION").split('.').next().unwrap()
    {
        // Panic if it's not
        error!("Unsupported manifest version.");
        panic!("Unsupported manifest version.");
    }

    info!("version is fine");

    // Get the DiskBlockStores
    let content_store: &CarBlockStore = &manifest_data.content_store;
    let meta_store: &CarBlockStore = &manifest_data.meta_store;

    // Deserialize the PrivateRef
    let dir_ref: PrivateRef = meta_store
        .get_deserializable(&manifest_data.ref_cid)
        .await
        .unwrap();

    info!("dir ref is fine");

    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = meta_store
        .get_deserializable(&manifest_data.ipld_cid)
        .await
        .unwrap();

    info!("forest ipld is fine");

    // Create a PrivateForest from that IPLD DAG
    let forest: Rc<PrivateForest> =
        Rc::new(ipld_serde::from_ipld::<PrivateForest>(forest_ipld).unwrap());

    info!("forest is fine");

    // Load the PrivateDirectory from the PrivateForest
    let dir: Rc<PrivateDirectory> = PrivateNode::load(&dir_ref, &forest, content_store)
        .await
        .unwrap()
        .as_dir()
        .unwrap();

    info!("dir is fine");

    Ok((forest, dir))
}
