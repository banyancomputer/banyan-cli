use anyhow::Result;
use rand::thread_rng;
use std::{
    io::{Read, Write},
    path::Path,
    rc::Rc,
};
use wnfs::{
    common::{dagcbor, AsyncSerialize, BlockStore, HashOutput},
    libipld::{serde as ipld_serde, Cid, Ipld},
    private::{AesKey, PrivateDirectory, PrivateForest, PrivateNode, PrivateRef, TemporalKey},
};

use tomb_common::types::pipeline::Manifest;

/// Store a Manifest
pub fn store_manifest(tomb_path: &Path, manifest: &Manifest) -> Result<()> {
    // The path in which we expect to find the Manifest JSON file
    let manifest_file = tomb_path.join("manifest.cbor");

    // For now just write out the content of compressed_and_encrypted to a file.
    // make sure the manifest file doesn't exist
    let mut manifest_writer = match std::fs::OpenOptions::new()
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

    // Write the manifest in DAG CBOR
    manifest_writer
        .write_all(&dagcbor::encode(&manifest)?)
        .map_err(anyhow::Error::new)
}

/// Deserializes the Manifest struct from a given .tomb dir
pub fn load_manifest(tomb_path: &Path) -> Result<Manifest> {
    info!("Loading in Manifest from disk");
    let manifest_file = tomb_path.join("manifest.cbor");

    // Read in the manifest file from the metadata path
    let mut manifest_reader = std::fs::File::open(manifest_file)
        .map_err(|e| anyhow::anyhow!("Failed to open manifest file: {}", e))?;

    let mut manifest_buf: Vec<u8> = Vec::new();
    manifest_reader.read_to_end(&mut manifest_buf)?;

    // Deserialize the data read as the latest version of manifestdata
    let manifest: Manifest = match dagcbor::decode(&manifest_buf) {
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
        panic!(
            "Unsupported manifest version. Using {} but found {}",
            env!("CARGO_PKG_VERSION"),
            manifest.version
        );
    }

    Ok(manifest)
}

/// Store a TemporalKey
pub fn store_key(tomb_path: &Path, temporal_key: &TemporalKey, label: &str) -> Result<()> {
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
pub fn load_key(tomb_path: &Path, label: &str) -> Result<TemporalKey> {
    info!("Loading in {} Key from disk", label);
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
    let hot_local = &manifest.hot_local;
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(hot_local).await?;

    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = hot_local.put_serializable(&forest_ipld).await?;
    // Add PrivateForest associated roots to meta store
    hot_local.insert_root("ipld_cid", ipld_cid);
    // Return Ok
    Ok(())
}

/// Load a PrivateForest
pub async fn load_forest(manifest: &Manifest) -> Result<Rc<PrivateForest>> {
    info!("Loading in PrivateForest from disk");

    let ipld_cid = &manifest.hot_local.get_root("ipld_cid")?;

    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = manifest.hot_local.get_deserializable(ipld_cid).await?;
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
    let hot_local = &manifest.hot_local;

    // Random number generator
    let rng = &mut thread_rng();

    // Store the root of the PrivateDirectory in the PrivateForest, retrieving a PrivateRef to it
    let dir_ref: PrivateRef = dir.store(forest, hot_local, rng).await?;

    // Extract the component fields of the PrivateDirectory's PrivateReference
    let PrivateRef {
        saturated_name_hash,
        temporal_key,
        content_cid,
    } = dir_ref;

    // Store it in the Metadata CarBlockStore
    let ref_cid = hot_local
        .put_serializable::<(HashOutput, Cid)>(&(saturated_name_hash, content_cid))
        .await?;

    // Add PrivateDirectory associated roots to meta store
    hot_local.insert_root(cid_key, ref_cid);

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
    info!("Loading in PrivateDirectory from disk");
    // Extract BlockStores
    let hot_local = &manifest.hot_local;

    // Get the PrivateRef CID
    let ref_cid = hot_local.get_root(cid_key)?;

    // Construct the saturated name hash
    let (saturated_name_hash, content_cid): (HashOutput, Cid) = hot_local
        .get_deserializable::<(HashOutput, Cid)>(&ref_cid)
        .await?;

    // Reconstruct the PrivateRef
    let dir_ref: PrivateRef =
        PrivateRef::with_temporal_key(saturated_name_hash, key.clone(), content_cid);

    // Load the PrivateDirectory from the PrivateForest
    let dir: Rc<PrivateDirectory> = PrivateNode::load(&dir_ref, forest, hot_local)
        .await?
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
    store_manifest(tomb_path, manifest)?;
    store_key(tomb_path, &temporal_key, "root")?;
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
    let key = load_key(tomb_path, "root")?;
    let manifest = load_manifest(tomb_path)?;
    let forest = load_forest(&manifest).await?;
    let dir = load_dir(&manifest, &key, &forest, "current_root").await?;
    Ok((key, manifest, forest, dir))
}

#[cfg(test)]
mod test {
    use crate::utils::{
        fs::ensure_path_exists_and_is_dir,
        serialize::{
            load_dir, load_forest, load_key, load_manifest, load_pipeline, store_dir, store_forest,
            store_key, store_manifest, store_pipeline,
        },
    };
    use anyhow::Result;
    use chrono::Utc;
    use rand::thread_rng;
    use std::{fs, path::PathBuf, rc::Rc};
    use tomb_common::types::{
        blockstore::{carblockstore::CarBlockStore, networkblockstore::NetworkBlockStore},
        pipeline::Manifest,
    };
    use wnfs::{
        namefilter::Namefilter,
        private::{PrivateDirectory, PrivateForest},
    };

    // Create all of the relevant objects, using real CarBlockStores and real data
    async fn setup(
        test_name: &str,
    ) -> Result<(PathBuf, Manifest, Rc<PrivateForest>, Rc<PrivateDirectory>)> {
        let path = PathBuf::from(test_name);
        ensure_path_exists_and_is_dir(&path)?;

        let content_path = path.join("content");
        let tomb_path = path.join(".tomb");

        let cold_local = CarBlockStore::new(&content_path, None);
        let hot_local = CarBlockStore::new(&tomb_path, None);

        let rng = &mut thread_rng();
        let mut root_dir = Rc::new(PrivateDirectory::new(
            Namefilter::default(),
            Utc::now(),
            rng,
        ));
        let mut forest = Rc::new(PrivateForest::new());
        // Open new file
        let file = root_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                &mut forest,
                &hot_local,
                rng,
            )
            .await?;
        // Set file content
        file.set_content(
            Utc::now(),
            "Hello Kitty!".as_bytes(),
            &mut forest,
            &cold_local,
            rng,
        )
        .await?;
        // Default remote endpoint
        let cold_remote = NetworkBlockStore::new("http://127.0.0.1", 5001);
        //
        let manifest_data = Manifest {
            version: "1.1.0".to_string(),
            cold_local,
            cold_remote,
            hot_local,
        };

        Ok((tomb_path, manifest_data, forest, root_dir))
    }

    // Delete the temporary directory
    async fn teardown(test_name: &str) -> Result<()> {
        let path = PathBuf::from(test_name);
        fs::remove_dir_all(path)?;
        Ok(())
    }

    #[tokio::test]
    async fn serial_key() -> Result<()> {
        // Start er up!
        let (tomb_path, manifest, mut forest, dir) = setup("serial_key").await?;

        // Generate key for this directory
        let key = store_dir(&manifest, &mut forest, &dir, "dir").await?;

        // Store and load
        store_key(&tomb_path, &key, "root")?;
        let new_key = load_key(&tomb_path, "root")?;

        // Assert equality
        assert_eq!(key, new_key);

        // Teardown
        teardown("serial_key").await
    }

    #[tokio::test]
    async fn serial_manifest() -> Result<()> {
        // Start er up!
        let (tomb_path, manifest, _, _) = setup("serial_manifest").await?;

        // Store and load
        store_manifest(&tomb_path, &manifest)?;
        let new_manifest = load_manifest(&tomb_path)?;

        // Assert equality
        assert_eq!(manifest, new_manifest);

        // Teardown
        teardown("serial_manifest").await
    }

    #[tokio::test]
    async fn serial_forest() -> Result<()> {
        // Start er up!
        let (_, manifest, mut forest, _) = setup("serial_forest").await?;

        // Store and load
        store_forest(&manifest, &mut forest).await?;
        let new_forest = load_forest(&manifest).await?;

        // Assert equality
        assert_eq!(
            new_forest.diff(&forest, &manifest.cold_local).await?.len(),
            0
        );

        // Teardown
        teardown("serial_forest").await
    }

    #[tokio::test]
    async fn serial_dir() -> Result<()> {
        // Start er up!
        let (_, manifest, mut forest, dir) = setup("serial_dir").await?;

        let key = store_dir(&manifest, &mut forest, &dir, "dir").await?;
        store_forest(&manifest, &mut forest).await?;
        let new_forest = load_forest(&manifest).await?;
        let new_dir = load_dir(&manifest, &key, &new_forest, "dir").await?;
        // Assert equality
        assert_eq!(dir, new_dir);

        // Teardown
        teardown("serial_dir").await
    }

    #[tokio::test]
    async fn serial_pipeline() -> Result<()> {
        // Start er up!
        let (tomb_path, manifest, mut forest, dir) = setup("serial_pipeline").await?;

        // Store and load
        let key = store_pipeline(&tomb_path, &manifest, &mut forest, &dir).await?;
        let (new_key, new_manifest, new_forest, new_dir) = load_pipeline(&tomb_path).await?;

        // Assert equality
        assert_eq!(new_key, key);
        assert_eq!(new_manifest, manifest);
        assert_eq!(
            new_forest
                .diff(&forest, &new_manifest.cold_local)
                .await?
                .len(),
            0
        );
        assert_eq!(new_dir, dir);

        // Teardown
        teardown("serial_pipeline").await
    }

    #[tokio::test]
    async fn serial_dir_content() -> Result<()> {
        // Start er up!
        let (_, manifest, mut original_forest, mut original_dir) =
            setup("serial_dir_content").await?;
        // Grab the original file
        let original_file = original_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                &mut original_forest,
                &manifest.hot_local,
                &mut thread_rng(),
            )
            .await?;
        // Get the content
        let original_content = original_file
            .get_content(&mut original_forest, &manifest.cold_local)
            .await?;

        let key = store_dir(&manifest, &mut original_forest, &original_dir, "dir").await?;
        store_forest(&manifest, &mut original_forest).await?;

        let mut new_forest = load_forest(&manifest).await?;
        let mut new_dir = load_dir(&manifest, &key, &new_forest, "dir").await?;
        // Assert equality
        assert_eq!(original_dir, new_dir);

        let file = new_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                &mut new_forest,
                &manifest.hot_local,
                &mut thread_rng(),
            )
            .await?;
        // Get the content
        let new_content = file
            .get_content(&mut new_forest, &manifest.cold_local)
            .await?;

        assert_eq!(original_content, new_content);

        // Teardown
        teardown("serial_dir_content").await
    }
}
