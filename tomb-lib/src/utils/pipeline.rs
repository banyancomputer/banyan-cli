use anyhow::Result;
use rand::thread_rng;
use std::{
    io::{Read, Write},
    path::Path,
    rc::Rc,
};
use tokio::io::AsyncWriteExt;
use wnfs::{
    common::{AsyncSerialize, BlockStore, HashOutput},
    libipld::{serde as ipld_serde, Ipld, Cid},
    private::{AesKey, PrivateDirectory, PrivateForest, PrivateNode, PrivateRef, TemporalKey},
};

use crate::types::pipeline::ManifestData;
use tempfile as _;

/// Dw about it
pub async fn store_manifest_and_key(
    tomb_path: &Path,
    temporal_key: &TemporalKey,
    manifest_data: &ManifestData,
) -> Result<()> {
    info!("Loading in cached metadata...");
    // The path in which we expect to find the Manifest JSON file
    let key_file = tomb_path.join("root.key");
    let manifest_file = tomb_path.join("manifest.json");

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

    // Use serde to convert the ManifestData to JSON and write it to the path specified
    serde_json::to_writer_pretty(manifest_writer, &manifest_data)
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

/// Deserializes the ManifestData struct from a given .tomb dir
pub async fn load_manifest_and_key(tomb_path: &Path) -> Result<(TemporalKey, ManifestData)> {
    info!("Loading in cached metadata...");
    // The path in which we expect to find the Manifest JSON file
    let key_file = tomb_path.join("root.key");
    let manifest_file = tomb_path.join("manifest.json");

    // Read in the key file from the key path
    let mut key_reader = std::fs::File::open(key_file)
        .map_err(|e| anyhow::anyhow!("Failed to open key file: {}", e))?;
    // Deserialize the data read as the latest version of manifestdata
    let mut key_data: [u8; 32] = [0; 32];
    key_reader.read_exact(&mut key_data)?;
    let key: TemporalKey = TemporalKey(AesKey::new(key_data));

    // Read in the manifest file from the metadata path
    let manifest_reader = std::fs::File::open(manifest_file)
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

    println!("loade: the key is {:?}", key);

    Ok((key, manifest_data))
}

/// Store the PrivateForest and PrivateDirectory in the content BlockStore
/// Return the CIDs of the references to those objects, which can be looked up in the Metadata BlockStore
pub async fn store_forest(
    manifest_data: &ManifestData,
    forest: &mut Rc<PrivateForest>,
) -> Result<()> {
    // Extract BlockStores
    let content_store = &manifest_data.content_store;
    let meta_store = &manifest_data.meta_store;
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(content_store).await?;
    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = meta_store.put_serializable(&forest_ipld).await?;

    println!("stored ipld with cid {} form content store", ipld_cid);

    // Add PrivateForest associated roots to meta store
    meta_store.insert_root("ipld_cid", ipld_cid);
    // Return Ok
    Ok(())
}

/// Loads in the PrivateForest and PrivateDirectory from a given ManifestData
pub async fn load_forest(manifest_data: &ManifestData) -> Result<Rc<PrivateForest>> {
    info!("Loading in Key, BlockStores, & WNFS from metadata...");

    let ipld_cid = &manifest_data.meta_store.get_root("ipld_cid")?;

    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = manifest_data
        .meta_store
        .get_deserializable(ipld_cid)
        .await?;
    // Create a PrivateForest from that IPLD DAG
    let forest: Rc<PrivateForest> =
        Rc::new(ipld_serde::from_ipld::<PrivateForest>(forest_ipld).unwrap());

    // Return both
    Ok(forest)
}

/// Store dir
pub async fn store_dir(
    manifest_data: &ManifestData,
    forest: &mut Rc<PrivateForest>,
    dir: &Rc<PrivateDirectory>,
) -> Result<TemporalKey> {
    // Extract BlockStores
    let content_store = &manifest_data.content_store;
    let meta_store = &manifest_data.meta_store;

    // Random number generator
    let rng = &mut thread_rng();

    // Store the root of the PrivateDirectory in the PrivateForest, retrieving a PrivateRef to it
    let dir_ref: PrivateRef = dir.store(forest, content_store, rng).await?;

    println!("pre_serial ref: {:?}", dir_ref);

    // Extract the component fields of the PrivateDirectory's PrivateReference
    let PrivateRef {
        saturated_name_hash,
        temporal_key,
        content_cid,
    } = dir_ref;

    println!("\nSHp: {:?}", saturated_name_hash);

    // Store it in the Metadata CarBlockStore
    let ref_cid = meta_store
        .put_serializable::<(HashOutput, Cid)>(&(saturated_name_hash, content_cid))
        .await?;

    // Add PrivateDirectory associated roots to meta store
    meta_store.insert_root("ref_cid", ref_cid);
    println!("store: the key is {:?}", temporal_key);
    // Return OK
    Ok(temporal_key)
}

/// Load dir
pub async fn load_dir(
    manifest_data: &ManifestData,
    key: TemporalKey,
    forest: &Rc<PrivateForest>,
) -> Result<Rc<PrivateDirectory>> {
    // Extract BlockStores
    let content_store = &manifest_data.content_store;
    let meta_store = &manifest_data.meta_store;

    // Get the PrivateRef CID
    let ref_cid = meta_store.get_root("ref_cid")?;

    // Construct the saturated name hash
    let (saturated_name_hash, content_cid): (HashOutput, Cid) = meta_store
        .get_deserializable::<(HashOutput, Cid)>(&ref_cid)
        .await?;

    println!("\nSHr: {:?}", saturated_name_hash);

    // Reconstruct the PrivateRef
    let dir_ref: PrivateRef = PrivateRef::with_temporal_key(saturated_name_hash, key, content_cid);

    println!("reconstructed ref: {:?}", dir_ref);

    // Load the PrivateDirectory from the PrivateForest
    let dir: Rc<PrivateDirectory> = PrivateNode::load(&dir_ref, &forest, content_store)
        .await
        .unwrap()
        .as_dir()?;

    Ok(dir)
}

///
pub async fn store_pipeline(
    tomb_path: &Path,
    manifest_data: &ManifestData,
    forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<TemporalKey> {
    // Store the dir, then the forest, then the manifest and key
    let temporal_key = store_dir(manifest_data, forest, root_dir).await?;
    store_forest(manifest_data, forest).await?;
    store_manifest_and_key(&tomb_path, &temporal_key, &manifest_data).await?;
    Ok(temporal_key)
}

///
pub async fn load_pipeline(
    tomb_path: &Path,
) -> Result<(ManifestData, Rc<PrivateForest>, Rc<PrivateDirectory>)> {
    let (key, manifest) = load_manifest_and_key(&tomb_path).await?;
    let forest = load_forest(&manifest).await?;
    let dir = load_dir(&manifest, key, &forest).await?;
    Ok((manifest, forest, dir))
}

/*
#[cfg(test)]
mod test {
    use std::{collections::HashSet, path::PathBuf, rc::Rc};

    use anyhow::Result;
    use chrono::Utc;
    use rand::thread_rng;
    use tempfile::tempdir;
    use wnfs::{
        libipld::Cid,
        namefilter::Namefilter,
        private::{PrivateDirectory, PrivateForest},
    };

    use crate::{
        types::{blockstore::carblockstore::CarBlockStore, pipeline::ManifestData},
        utils::pipeline::{
            load_dir, load_forest, load_manifest_and_key, load_pipeline, store_dir, store_forest,
            store_manifest_and_key, store_pipeline,
        },
    };

    async fn setup() -> Result<(
        PathBuf,
        ManifestData,
        Rc<PrivateForest>,
        Rc<PrivateDirectory>,
    )> {
        let dir = tempdir()?;
        let content_path = dir.path().join("content");
        let tomb_path = dir.path().join(".tomb");

        let content_store = CarBlockStore::new(&content_path, None);
        let meta_store = CarBlockStore::new(&tomb_path, None);

        let rng = &mut thread_rng();
        let mut root_dir = Rc::new(PrivateDirectory::new(
            Namefilter::default(),
            Utc::now(),
            rng,
        ));
        let mut forest = Rc::new(PrivateForest::new());
        root_dir
            .write(
                &["cats".to_string()],
                true,
                Utc::now(),
                b"".to_vec(),
                &mut forest,
                &content_store,
                rng,
            )
            .await?;

        let manifest_data = ManifestData {
            version: "1.1.0".to_string(),
            content_store,
            meta_store,
        };

        Ok((tomb_path, manifest_data, forest, root_dir))
    }

    #[tokio::test]
    async fn test_serialiasdfasze() -> Result<()> {
        // Start er up!
        let (tomb_path, manifest_data, mut forest, root_dir) = setup().await?;

        // Store everything
        store_pipeline(&tomb_path, &manifest_data, &mut forest, &root_dir).await?;

        // Load everything
        let (new_manifest, new_forest, new_dir) = load_pipeline(&tomb_path).await?;

        Ok(())
    }
    
    #[tokio::test]
    async fn test_serial_dir() -> Result<()> {
        // Start er up!
        let (_tomb_path, manifest_data, mut forest, root_dir) = setup().await?;

        // Store the directory in the content store
        let temporal_key = store_dir(&manifest_data, &mut forest, &root_dir).await?;
        // Load the idrectory from the content store
        let dir = load_dir(&manifest_data, temporal_key, &forest).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_serialization_pipeline() -> Result<()> {
        // Start er up!
        let (tomb_path, manifest_data, mut forest, root_dir) = setup().await?;

        let temporal_key = store_dir(&manifest_data, &mut forest, &root_dir).await?;
        store_forest(&manifest_data, &mut forest).await?;
        store_manifest_and_key(&tomb_path, &temporal_key, &manifest_data).await?;

        /*
        // Ensure key equality
        let (new_key, new_manifest) = load_manifest_and_key(&tomb_path).await?;
        assert_eq!(new_key, temporal_key);

        // Ensure content BlockStore equality
        let old_content: HashSet<Cid> =
            HashSet::from_iter(manifest_data.content_store.get_all_cids());
        let new_content: HashSet<Cid> =
            HashSet::from_iter(new_manifest.content_store.get_all_cids());
        assert_eq!(old_content, new_content);

        // Ensure meta BlockStore equality
        let old_meta: HashSet<Cid> = HashSet::from_iter(manifest_data.meta_store.get_all_cids());
        let new_meta: HashSet<Cid> = HashSet::from_iter(new_manifest.meta_store.get_all_cids());
        assert_eq!(old_meta, new_meta);

        // Ensure PrivateForest equality
        let new_forest = load_forest(&new_manifest).await?;
        let forest_difference = new_forest
            .diff(&forest, &new_manifest.content_store)
            .await?;
        assert!(forest_difference.len() == 0);

        //
        let new_dir = load_dir(&new_manifest, new_key, &new_forest).await?;

         */
        Ok(())
    }
}
 */