use crate::types::{blockstore::car::carv2::carv2blockstore::CarV2BlockStore, pipeline::Manifest};
use anyhow::Result;
use rand::thread_rng;
use std::rc::Rc;
use wnfs::{
    common::{AsyncSerialize, BlockStore, HashOutput},
    libipld::{serde as ipld_serde, Cid, Ipld},
    private::{PrivateDirectory, PrivateForest, PrivateNode, PrivateRef, TemporalKey},
};

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
pub async fn store_metadata_forest(
    metadata: &CarV2BlockStore,
    metadata_forest: &Rc<PrivateForest>,
) -> Result<()> {
    // Store the forest in the hot store
    let hot_cid = store_forest(metadata_forest, metadata).await?;
    // Add PrivateForest associated roots to meta store
    metadata.insert_root(&hot_cid)?;
    // Return Ok
    Ok(())
}

/// Load the hot PrivateForest
pub async fn load_metadata_forest(metadata: &CarV2BlockStore) -> Result<Rc<PrivateForest>> {
    // Get the CID from the hot store
    let metadata_cid = &metadata.get_roots()[1];
    // Load the forest
    load_forest(metadata_cid, metadata).await
}

/// Store the cold PrivateForest
pub async fn store_content_forest(
    content: &CarV2BlockStore,
    content_forest: &Rc<PrivateForest>,
) -> Result<()> {
    // Store the forest in the hot store
    let content_cid = store_forest(content_forest, content).await?;
    // Add PrivateForest associated roots to meta store
    content.insert_root(&content_cid)?;
    // Return Ok
    Ok(())
}

/// Load the cold PrivateForest
pub async fn load_content_forest(content: &CarV2BlockStore) -> Result<Rc<PrivateForest>> {
    // Get the CID from the hot store
    let content_cid = &content.get_roots()[2];
    // Load the forest
    load_forest(content_cid, content).await
}

/// Store a PrivateDirectory
pub async fn store_dir(
    manifest: &mut Manifest,
    metadata_forest: &mut Rc<PrivateForest>,
    dir: &Rc<PrivateDirectory>,
) -> Result<TemporalKey> {
    // Random number generator
    let rng = &mut thread_rng();

    // Store the root of the PrivateDirectory in the PrivateForest, retrieving a PrivateRef to it
    let dir_ref: PrivateRef = dir.store(metadata_forest, &manifest.metadata, rng).await?;

    // Extract the component fields of the PrivateDirectory's PrivateReference
    let PrivateRef {
        saturated_name_hash,
        temporal_key,
        content_cid,
    } = dir_ref;

    // Store it in the Metadata BlockStore
    let ref_cid = manifest
        .metadata
        .put_serializable::<(HashOutput, Cid)>(&(saturated_name_hash, content_cid))
        .await?;

    // Add PrivateDirectory associated roots to meta store
    manifest.metadata.insert_root(&ref_cid)?;

    // Return OK
    Ok(temporal_key)
}

/// Load a PrivateDirectory
pub async fn load_dir(
    manifest: &Manifest,
    key: &TemporalKey,
    metadata_forest: &Rc<PrivateForest>,
) -> Result<Rc<PrivateDirectory>> {
    // Get the PrivateRef CID
    let ref_cid = &manifest.metadata.get_roots()[0];

    // Construct the saturated name hash
    let (saturated_name_hash, content_cid): (HashOutput, Cid) = manifest
        .metadata
        .get_deserializable::<(HashOutput, Cid)>(ref_cid)
        .await?;

    // Reconstruct the PrivateRef
    let dir_ref: PrivateRef =
        PrivateRef::with_temporal_key(saturated_name_hash, key.clone(), content_cid);

    // Load the PrivateDirectory from the PrivateForest
    PrivateNode::load(&dir_ref, metadata_forest, &manifest.metadata)
        .await?
        .as_dir()
}

/// Store all hot objects!
pub async fn store_all_hot(
    manifest: &mut Manifest,
    metadata_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<TemporalKey> {
    // Store the dir, then the forest, then the manifest and key
    let temporal_key = store_dir(manifest, metadata_forest, root_dir).await?;
    store_metadata_forest(&manifest.metadata, metadata_forest).await?;
    Ok(temporal_key)
}

/// Load all hot objects!
pub async fn load_all_hot(
    key: &TemporalKey,
    manifest: &Manifest,
) -> Result<(Rc<PrivateForest>, Rc<PrivateDirectory>)> {
    let metadata_forest = load_metadata_forest(&manifest.metadata).await?;
    let dir = load_dir(manifest, key, &metadata_forest).await?;
    Ok((metadata_forest, dir))
}

/// Store everything at once!
pub async fn store_all(
    manifest: &mut Manifest,
    metadata_forest: &mut Rc<PrivateForest>,
    content_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<TemporalKey> {
    // Empty all roots first
    manifest.metadata.empty_roots()?;

    let temporal_key = store_dir(manifest, metadata_forest, root_dir).await?;

    store_metadata_forest(&manifest.metadata, metadata_forest).await?;
    store_content_forest(&manifest.content, content_forest).await?;

    Ok(temporal_key)
}

/// Load everything at once!
pub async fn load_all(
    key: &TemporalKey,
    manifest: &Manifest,
) -> Result<(Rc<PrivateForest>, Rc<PrivateForest>, Rc<PrivateDirectory>)> {
    let (metadata_forest, content_forest) = (
        load_metadata_forest(&manifest.metadata).await?,
        load_content_forest(&manifest.content).await?,
    );
    let dir = load_dir(manifest, key, &metadata_forest).await?;
    Ok((metadata_forest, content_forest, dir))
}
