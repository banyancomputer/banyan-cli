use anyhow::Result;
use rand::thread_rng;
use std::{
    io::{Read, Write},
    path::Path,
    rc::Rc,
};
use wnfs::{
    common::{AsyncSerialize, BlockStore, HashOutput},
    libipld::{serde as ipld_serde, Cid, Ipld},
    private::{AesKey, PrivateDirectory, PrivateForest, PrivateNode, PrivateRef, TemporalKey},
};

use crate::types::pipeline::Manifest;
use tempfile as _;

/// Store a Manifest
pub async fn store_manifest(tomb_path: &Path, manifest: &Manifest) -> Result<()> {
    // The path in which we expect to find the Manifest JSON file
    let manifest_file = tomb_path.join("manifest.json");

    // For now just write out the content of compressed_and_encrypted to a file.
    // make sure the manifest file doesn't exist
    let manifest_writer = match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&manifest_file)
    {
        Ok(f) => f,
        Err(e) => Err(anyhow::anyhow!(
            "Failed to create manifest file at {}: {}",
            manifest_file.display(),
            e
        ))?,
    };

    info!(
        "📄 Writing out a data manifest file to {}",
        manifest_file.display()
    );

    // Use serde to convert the Manifest to JSON and write it to the path specified
    serde_json::to_writer_pretty(manifest_writer, &manifest).map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

/// Deserializes the Manifest struct from a given .tomb dir
pub async fn load_manifest(tomb_path: &Path) -> Result<Manifest> {
    let manifest_file = tomb_path.join("manifest.json");

    // Read in the manifest file from the metadata path
    let manifest_reader = std::fs::File::open(manifest_file)
        .map_err(|e| anyhow::anyhow!("Failed to open manifest file: {}", e))?;
    // Deserialize the data read as the latest version of manifestdata
    let manifest: Manifest = match serde_json::from_reader(manifest_reader) {
        Ok(data) => data,
        Err(e) => {
            panic!("Failed to deserialize manifest file: {e}");
        }
    };

    // If the major version of the manifest is not the same as the major version of the program
    if manifest.version.split('.').next().unwrap()
        != env!("CARGO_PKG_VERSION").split('.').next().unwrap()
    {
        // Panic if it's not
        panic!("Unsupported manifest version.");
    }

    Ok(manifest)
}

/// Store a TemporalKey
pub async fn store_key(tomb_path: &Path, temporal_key: &TemporalKey, label: &str) -> Result<()> {
    // The path in which we expect to find the Manifest JSON file
    let key_file = tomb_path.join(format!("{}.key", label));
    let mut key_writer = match std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&key_file)
    {
        Ok(f) => f,
        Err(e) => Err(anyhow::anyhow!(
            "Failed to create key file at {}: {}",
            key_file.display(),
            e
        ))?,
    };

    // Write the key
    key_writer.write_all(temporal_key.0.as_bytes())?;

    Ok(())
}

/// Load a TemporalKey
pub async fn load_key(tomb_path: &Path, label: &str) -> Result<TemporalKey> {
    // The path in which we expect to find the Manifest JSON file
    let key_file = tomb_path.join(format!("{}.key", label));

    // Read in the key file from the key path
    let mut key_reader = std::fs::File::open(key_file)
        .map_err(|e| anyhow::anyhow!("Failed to open key file: {}", e))?;
    // Deserialize the data read as the latest version of manifestdata
    let mut key_data: [u8; 32] = [0; 32];
    key_reader.read_exact(&mut key_data)?;
    let key: TemporalKey = TemporalKey(AesKey::new(key_data));

    Ok(key)
}

/// Store a PrivateForest
pub async fn store_forest(manifest: &Manifest, forest: &mut Rc<PrivateForest>) -> Result<()> {
    // Extract BlockStores
    let content_store = &manifest.content_store;
    let meta_store = &manifest.meta_store;
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(content_store).await?;
    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = meta_store.put_serializable(&forest_ipld).await?;
    // Add PrivateForest associated roots to meta store
    meta_store.insert_root("ipld_cid", ipld_cid);
    // Return Ok
    Ok(())
}

/// Load a PrivateForest
pub async fn load_forest(manifest: &Manifest) -> Result<Rc<PrivateForest>> {
    info!("Loading in Key, BlockStores, & WNFS from metadata...");

    let ipld_cid = &manifest.meta_store.get_root("ipld_cid")?;

    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = manifest.meta_store.get_deserializable(ipld_cid).await?;
    // Create a PrivateForest from that IPLD DAG
    let forest: Rc<PrivateForest> =
        Rc::new(ipld_serde::from_ipld::<PrivateForest>(forest_ipld).unwrap());

    // Return both
    Ok(forest)
}

/// Store a PrivateDirectory
pub async fn store_dir(
    manifest: &Manifest,
    forest: &mut Rc<PrivateForest>,
    dir: &Rc<PrivateDirectory>,
    cid_key: &str,
) -> Result<TemporalKey> {
    // Extract BlockStores
    let content_store = &manifest.content_store;
    let meta_store = &manifest.meta_store;

    // Random number generator
    let rng = &mut thread_rng();

    // Store the root of the PrivateDirectory in the PrivateForest, retrieving a PrivateRef to it
    let dir_ref: PrivateRef = dir.store(forest, content_store, rng).await?;

    // Extract the component fields of the PrivateDirectory's PrivateReference
    let PrivateRef {
        saturated_name_hash,
        temporal_key,
        content_cid,
    } = dir_ref;

    // Store it in the Metadata CarBlockStore
    let ref_cid = meta_store
        .put_serializable::<(HashOutput, Cid)>(&(saturated_name_hash, content_cid))
        .await?;

    // Add PrivateDirectory associated roots to meta store
    meta_store.insert_root(cid_key, ref_cid);

    // Return OK
    Ok(temporal_key)
}

/// Load a PrivateDirectory
pub async fn load_dir(
    manifest: &Manifest,
    key: &TemporalKey,
    forest: &Rc<PrivateForest>,
    cid_key: &str,
) -> Result<Rc<PrivateDirectory>> {
    // Extract BlockStores
    let content_store = &manifest.content_store;
    let meta_store = &manifest.meta_store;

    // Get the PrivateRef CID
    let ref_cid = meta_store.get_root(cid_key)?;

    // Construct the saturated name hash
    let (saturated_name_hash, content_cid): (HashOutput, Cid) = meta_store
        .get_deserializable::<(HashOutput, Cid)>(&ref_cid)
        .await?;

    // Reconstruct the PrivateRef
    let dir_ref: PrivateRef =
        PrivateRef::with_temporal_key(saturated_name_hash, key.clone(), content_cid);

    // Load the PrivateDirectory from the PrivateForest
    let dir: Rc<PrivateDirectory> = PrivateNode::load(&dir_ref, forest, content_store)
        .await
        .unwrap()
        .as_dir()?;

    Ok(dir)
}

/// Store everything at once!
pub async fn store_pipeline(
    tomb_path: &Path,
    manifest: &Manifest,
    forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<TemporalKey> {
    // Store the dir, then the forest, then the manifest and key
    let temporal_key = store_dir(manifest, forest, root_dir, "current_root").await?;
    store_forest(manifest, forest).await?;
    store_manifest(tomb_path, manifest).await?;
    store_key(tomb_path, &temporal_key, "root").await?;
    Ok(temporal_key)
}

/// Load everything at once!
pub async fn load_pipeline(
    tomb_path: &Path,
) -> Result<(
    TemporalKey,
    Manifest,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
)> {
    let key = load_key(tomb_path, "root").await?;
    let manifest = load_manifest(tomb_path).await?;
    let forest = load_forest(&manifest).await?;
    let dir = load_dir(&manifest, &key, &forest, "current_root").await?;
    Ok((key, manifest, forest, dir))
}
