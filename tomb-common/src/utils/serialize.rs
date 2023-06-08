use std::{rc::Rc, collections::HashMap};

use wnfs::{private::{TemporalKey, PrivateForest, PrivateDirectory, PrivateRef, PrivateNode}, common::{HashOutput, BlockStore, AsyncSerialize}, libipld::{serde as ipld_serde, Cid, Ipld}};
use crate::types::pipeline::Manifest;
use anyhow::Result;
use rand::thread_rng;


/// Store a given PrivateForest in a given Store
pub async fn store_forest(forest: &Rc<PrivateForest>, store: &impl BlockStore) -> Result<Cid> {
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(store).await?;
    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = store.put_serializable(&forest_ipld).await?;
    // Return Ok
    Ok(ipld_cid)
}

/// Load a given PrivateForest from a given Store
pub async fn load_forest(cid: &Cid, store: &impl BlockStore) -> Result<Rc<PrivateForest>> {
    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = store.get_deserializable(cid).await?;
    // Create a PrivateForest from that IPLD DAG
    let forest: Rc<PrivateForest> =
        Rc::new(ipld_serde::from_ipld::<PrivateForest>(forest_ipld).unwrap());
    // Return
    Ok(forest)
}

/// Store the hot PrivateForest
pub async fn store_hot_forest(
    map: &mut HashMap<String, Cid>,
    hot_store: &impl BlockStore,
    hot_forest: &Rc<PrivateForest>,
) -> Result<()> {
    // Store the forest in the hot store
    let hot_cid = store_forest(hot_forest, hot_store).await?;
    // Add PrivateForest associated roots to meta store
    map.insert(String::from("hot_ipld_cid"), hot_cid);
    // Return Ok
    Ok(())
}

/// Load the hot PrivateForest
pub async fn load_hot_forest(
    map: &HashMap<String, Cid>,
    hot_store: &impl BlockStore,
) -> Result<Rc<PrivateForest>> {
    // Get the CID from the hot store
    let hot_cid = map.get("hot_ipld_cid").unwrap();
    // Load the forest
    load_forest(hot_cid, hot_store).await
}

/// Store the cold PrivateForest
pub async fn store_cold_forest(
    map: &mut HashMap<String, Cid>,
    cold_store: &impl BlockStore,
    cold_forest: &Rc<PrivateForest>,
) -> Result<()> {
    // Store the forest in the hot store
    let cold_cid = store_forest(cold_forest, cold_store).await?;
    // Add PrivateForest associated roots to meta store
    map.insert(String::from("cold_ipld_cid"), cold_cid);
    // Return Ok
    Ok(())
}

/// Load the cold PrivateForest
pub async fn load_cold_forest(
    map: &HashMap<String, Cid>,
    cold_store: &impl BlockStore,
) -> Result<Rc<PrivateForest>> {
    // Get the CID from the hot store
    let hot_cid = &map.get("cold_ipld_cid").unwrap();
    // Load the forest
    load_forest(hot_cid, cold_store).await
}

/// Store a PrivateDirectory
pub async fn store_dir(
    manifest: &mut Manifest,
    hot_forest: &mut Rc<PrivateForest>,
    dir: &Rc<PrivateDirectory>,
    cid_key: &str,
) -> Result<TemporalKey> {
    // Extract BlockStores
    let hot_store = &manifest.hot_local;

    // Random number generator
    let rng = &mut thread_rng();

    // Store the root of the PrivateDirectory in the PrivateForest, retrieving a PrivateRef to it
    let dir_ref: PrivateRef = dir.store(hot_forest, hot_store, rng).await?;

    // Extract the component fields of the PrivateDirectory's PrivateReference
    let PrivateRef {
        saturated_name_hash,
        temporal_key,
        content_cid,
    } = dir_ref;

    // Store it in the Metadata BlockStore
    let ref_cid = hot_store
        .put_serializable::<(HashOutput, Cid)>(&(saturated_name_hash, content_cid))
        .await?;

    // Add PrivateDirectory associated roots to meta store
    manifest.roots.insert(cid_key.to_string(), ref_cid);

    // Return OK
    Ok(temporal_key)
}

/// Load a PrivateDirectory
pub async fn load_dir(
    manifest: &Manifest,
    key: &TemporalKey,
    hot_forest: &Rc<PrivateForest>,
    cid_key: &str,
) -> Result<Rc<PrivateDirectory>> {
    // Extract BlockStores
    let hot_local = &manifest.hot_local;

    // Get the PrivateRef CID
    let ref_cid = manifest.roots.get(cid_key).unwrap();

    // Construct the saturated name hash
    let (saturated_name_hash, content_cid): (HashOutput, Cid) = hot_local
        .get_deserializable::<(HashOutput, Cid)>(&ref_cid)
        .await?;

    // Reconstruct the PrivateRef
    let dir_ref: PrivateRef =
        PrivateRef::with_temporal_key(saturated_name_hash, key.clone(), content_cid);

    // Load the PrivateDirectory from the PrivateForest
    let dir: Rc<PrivateDirectory> = PrivateNode::load(&dir_ref, hot_forest, hot_local)
        .await?
        .as_dir()?;

    Ok(dir)
}

/// Store all hot objects!
pub async fn store_all_hot(
    mut manifest: Manifest,
    hot_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<(TemporalKey, Manifest)> {
    // Store the dir, then the forest, then the manifest and key
    let temporal_key = store_dir(&mut manifest, hot_forest, root_dir, "current_root").await?;
    store_hot_forest(&mut manifest.roots, &manifest.hot_local, hot_forest).await?;
    Ok((temporal_key, manifest))
}

/// Load all hot objects!
pub async fn load_all_hot(
    key: &TemporalKey,
    manifest: &Manifest
) -> Result<(
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
)> {
    let hot_forest = load_hot_forest(&manifest.roots, &manifest.hot_local).await?;
    let dir = load_dir(&manifest, &key, &hot_forest, "current_root").await?;
    Ok((hot_forest, dir))
}

/// Store everything at once!
pub async fn store_all(
    local: bool,
    mut manifest: Manifest,
    hot_forest: &mut Rc<PrivateForest>,
    cold_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<(TemporalKey, Manifest)> {
    let temporal_key = store_dir(&mut manifest, hot_forest, root_dir, "current_root").await?;
    
    if local {
        store_hot_forest(&mut manifest.roots, &manifest.hot_local, hot_forest).await?;
        store_cold_forest(&mut manifest.roots, &manifest.cold_local, cold_forest).await?;
    } else {
        store_hot_forest(&mut manifest.roots, &manifest.cold_remote, hot_forest).await?;
        store_cold_forest(&mut manifest.roots, &manifest.cold_remote, cold_forest).await?;
    }

    Ok((temporal_key, manifest))
}

/// Load everything at once!
pub async fn load_all(
    local: bool,
    key: &TemporalKey,
    manifest: &Manifest
) -> Result<(
    Rc<PrivateForest>,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
)> {
    let hot_forest = load_hot_forest(&manifest.roots, &manifest.hot_local).await?;
    let dir = load_dir(&manifest, &key, &hot_forest, "current_root").await?;

    let (hot_forest, cold_forest) = if local {
        (
            load_hot_forest(&manifest.roots, &manifest.hot_local).await?,
            load_cold_forest(&manifest.roots, &manifest.cold_local).await?
        )
    } else {
        (
            load_hot_forest(&manifest.roots, &manifest.hot_remote).await?,
            load_cold_forest(&manifest.roots, &manifest.cold_remote).await?
        )
    };
    Ok((hot_forest, cold_forest, dir))
}
