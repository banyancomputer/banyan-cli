use anyhow::Result;
use rand::thread_rng;
use std::{io::Read, path::Path, rc::Rc};
use wnfs::{
    common::{AsyncSerialize, BlockStore, HashOutput},
    libipld::{serde as ipld_serde, Cid, Ipld},
    private::{AesKey, PrivateDirectory, PrivateForest, PrivateNode, PrivateRef, TemporalKey},
};

use crate::types::{blockstore::carblockstore::CarBlockStore, pipeline::ManifestData};

/// Deserializes the ManifestData struct from a given .tomb dir
pub async fn load_manifest_and_key(input_meta_path: &Path) -> Result<(TemporalKey, ManifestData)> {
    info!("Loading in cached metadata...");
    // The path in which we expect to find the Manifest JSON file
    let key_file_path = input_meta_path.join("root.key");
    let meta_file_path = input_meta_path.join("manifest.json");

    // Read in the key file from the key path
    let mut key_reader = std::fs::File::open(key_file_path)
        .map_err(|e| anyhow::anyhow!("Failed to open key file: {}", e))?;
    // Deserialize the data read as the latest version of manifestdata
    let mut key_data: [u8; 32] = [0; 32];
    key_reader.read_exact(&mut key_data)?;
    let key: TemporalKey = TemporalKey(AesKey::new(key_data));

    // Read in the manifest file from the metadata path
    let manifest_reader = std::fs::File::open(meta_file_path)
        .map_err(|e| anyhow::anyhow!("Failed to open manifest file: {}", e))?;
    // Deserialize the data read as the latest version of manifestdata
    let manifest_data: ManifestData = match serde_json::from_reader(manifest_reader) {
        Ok(data) => data,
        Err(e) => {
            panic!("Failed to deserialize manifest file: {e}");
        }
    };

    // If the major version of the manifest is not the same as the major version of the program
    if manifest_data.version.split('.').next().unwrap()
        != env!("CARGO_PKG_VERSION").split('.').next().unwrap()
    {
        // Panic if it's not
        panic!("Unsupported manifest version.");
    }

    println!(
        "loade: the key is {:?} and the roots are {:?}",
        key,
        manifest_data.meta_store.get_roots()
    );

    Ok((key, manifest_data))
}

/// Loads in the PrivateForest and PrivateDirectory from a given ManifestData
pub async fn load_forest_and_dir(
    key: TemporalKey,
    manifest_data: &ManifestData,
) -> Result<(Rc<PrivateForest>, Rc<PrivateDirectory>)> {
    info!("Loading in Key, BlockStores, & WNFS from metadata...");

    // Get the DiskBlockStores
    let content_store: &CarBlockStore = &manifest_data.content_store;
    let meta_store: &CarBlockStore = &manifest_data.meta_store;
    // Get all the root CIDs from metadata store
    let roots: Vec<Cid> = meta_store.get_roots();

    // Construct the saturated name hash
    let saturated_name_hash: HashOutput = meta_store
        .get_deserializable::<HashOutput>(&roots[0])
        .await?;

    println!("\nSHr: {:?}", saturated_name_hash);

    // Reconstruct the PrivateRef
    let dir_ref: PrivateRef = PrivateRef::with_temporal_key(saturated_name_hash, key, roots[1]);

    println!("reconstructed ref: {:?}", dir_ref);

    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = meta_store.get_deserializable(&roots[2]).await?;

    // Create a PrivateForest from that IPLD DAG
    let forest: Rc<PrivateForest> =
        Rc::new(ipld_serde::from_ipld::<PrivateForest>(forest_ipld).unwrap());

    // Load the PrivateDirectory from the PrivateForest
    let dir: Rc<PrivateDirectory> = PrivateNode::load(&dir_ref, &forest, content_store)
        .await
        .unwrap()
        .as_dir()?;

    Ok((forest, dir))
}

/// Store the PrivateForest and PrivateDirectory in the content BlockStore
/// Return the CIDs of the references to those objects, which can be looked up in the Metadata BlockStore
pub async fn store_forest_and_dir(
    content_store: &mut CarBlockStore,
    meta_store: &mut CarBlockStore,
    forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<TemporalKey> {
    // Random number generator
    let rng = &mut thread_rng();
    // Store the root of the PrivateDirectory in the PrivateForest, retrieving a PrivateRef to it
    let root_ref: PrivateRef = root_dir.store(forest, content_store, rng).await?;

    println!("pre-serial ref: {:?}", root_ref);

    // Extract the component fields of the PrivateDirectory's PrivateReference
    let PrivateRef {
        saturated_name_hash,
        temporal_key,
        content_cid,
    } = root_ref;

    println!("\nSHp: {:?}", saturated_name_hash);

    // Store it in the Metadata CarBlockStore
    let hash_cid = meta_store
        .put_serializable::<HashOutput>(&saturated_name_hash)
        .await?;
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(content_store).await?;
    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = meta_store.put_serializable(&forest_ipld).await?;

    // Add PrivateDirectory associated roots to meta store
    meta_store.add_root(&hash_cid);
    meta_store.add_root(&content_cid);
    // Add PrivateForest associated roots to meta store
    meta_store.add_root(&ipld_cid);

    println!(
        "store: the key is {:?} and the roots are {:?}",
        temporal_key,
        meta_store.get_roots()
    );

    // Return OK
    Ok(temporal_key)
}
