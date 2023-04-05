use crate::types::unpack_plan::ManifestData;
use anyhow::Result;
// use serde::{Deserialize, Serializer};
use std::{path::Path, rc::Rc};
use wnfs::{
    common::{BlockStore, DiskBlockStore},
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

    // If the major version of the manifest is not the same as the major version of the program
    if manifest_data.version.split('.').next().unwrap()
        != env!("CARGO_PKG_VERSION").split('.').next().unwrap()
    {
        // Panic if it's not
        error!("Unsupported manifest version.");
        panic!("Unsupported manifest version.");
    }

    // Get the DiskBlockStore
    let store: DiskBlockStore = manifest_data.store;

    // Deserialize the PrivateRef
    let dir_ref: PrivateRef = store
        .get_deserializable(&manifest_data.ref_cid)
        .await
        .unwrap();

    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = store
        .get_deserializable(&manifest_data.ipld_cid)
        .await
        .unwrap();

    // Create a PrivateForest from that IPLD DAG
    let forest: PrivateForest = ipld_serde::from_ipld::<_>(forest_ipld)?;

    // Load the PrivateDirectory from the PrivateForest
    let _dir: Rc<PrivateDirectory> = PrivateNode::load(&dir_ref, &forest, &store)
        .await
        .unwrap()
        .as_dir()
        .unwrap();
    
    info!(
        "🔐 Decompressing and decrypting each file as it is copied to the new filesystem at {}",
        output_dir.display()
    );

    //TODO (organizedgrime) - implement the unpacking pipeline
    Ok(())
}
