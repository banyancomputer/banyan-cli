use crate::types::blockstore::car::carv2::carv2blockstore::CarV2BlockStore;
use anyhow::Result;
use rand::thread_rng;
use std::rc::Rc;
use wnfs::{
    common::{AsyncSerialize, BlockStore, HashOutput},
    libipld::{serde as ipld_serde, Cid, Ipld},
    private::{PrivateDirectory, PrivateForest, PrivateNode, PrivateRef, TemporalKey},
};

/// Store a given PrivateForest in a given Store
pub(crate) async fn store_forest(
    forest: &Rc<PrivateForest>,
    store: &impl BlockStore,
) -> Result<Cid> {
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
pub(crate) async fn store_metadata_forest(
    metadata: &CarV2BlockStore,
    metadata_forest: &Rc<PrivateForest>,
) -> Result<()> {
    // Store the forest in the hot store
    let metadata_cid = store_forest(metadata_forest, metadata).await?;
    println!("s: storing metadata_forest with cid {}", metadata_cid);
    // Add PrivateForest associated roots to meta store
    metadata.insert_root(&metadata_cid);
    // Return Ok
    Ok(())
}

/// Load the hot PrivateForest
async fn load_metadata_forest(metadata: &CarV2BlockStore, i: usize) -> Result<Rc<PrivateForest>> {
    // Get the CID from the hot store
    let metadata_cid = &metadata.get_roots()[i];
    println!("s: loading metadata_forest with cid {}", metadata_cid);
    // Load the forest
    load_forest(metadata_cid, metadata).await
}

/// Store the cold PrivateForest
async fn store_content_forest(
    content: &CarV2BlockStore,
    content_forest: &Rc<PrivateForest>,
) -> Result<()> {
    // Store the forest in the hot store
    let content_cid = store_forest(content_forest, content).await?;
    println!("s: storing contnet_forest with cid {}", content_cid);
    // Add PrivateForest associated roots to meta store
    content.insert_root(&content_cid);
    // Return Ok
    Ok(())
}

/// Load the cold PrivateForest
async fn load_content_forest(content: &CarV2BlockStore, i: usize) -> Result<Rc<PrivateForest>> {
    // Get the CID from the hot store
    let content_cid = &content.get_roots()[i];
    println!("s: loading contnet_forest with cid {}", content_cid);
    // Load the forest
    load_forest(content_cid, content).await
}

/// Store a PrivateDirectory
pub(crate) async fn store_dir(
    metadata: &CarV2BlockStore,
    metadata_forest: &mut Rc<PrivateForest>,
    dir: &Rc<PrivateDirectory>,
) -> Result<TemporalKey> {
    // Random number generator
    let rng = &mut thread_rng();

    // Store the root of the PrivateDirectory in the PrivateForest, retrieving a PrivateRef to it
    let dir_ref: PrivateRef = dir.store(metadata_forest, metadata, rng).await?;

    // Extract the component fields of the PrivateDirectory's PrivateReference
    let PrivateRef {
        saturated_name_hash,
        temporal_key,
        content_cid,
    } = dir_ref;

    // Store it in the Metadata BlockStore
    let ref_cid = metadata
        .put_serializable::<(HashOutput, Cid)>(&(saturated_name_hash, content_cid))
        .await?;

    // Add PrivateDirectory associated roots to meta store
    metadata.insert_root(&ref_cid);

    // Return OK
    Ok(temporal_key)
}

/// Load a PrivateDirectory
pub async fn load_dir(
    metadata: &CarV2BlockStore,
    key: &TemporalKey,
    metadata_forest: &Rc<PrivateForest>,
) -> Result<Rc<PrivateDirectory>> {
    // Get the PrivateRef CID
    let ref_cid = &metadata.get_roots()[0];
    println!("loading PrivateNode dir with cid {}", ref_cid);

    // Construct the saturated name hash
    let (saturated_name_hash, content_cid): (HashOutput, Cid) = metadata
        .get_deserializable::<(HashOutput, Cid)>(ref_cid)
        .await?;

    // Reconstruct the PrivateRef
    let dir_ref: PrivateRef =
        PrivateRef::with_temporal_key(saturated_name_hash, key.clone(), content_cid);

    // Load the PrivateDirectory from the PrivateForest
    PrivateNode::load(&dir_ref, metadata_forest, metadata)
        .await?
        .as_dir()
}

/// Store all hot objects!
pub async fn store_all_metadata(
    metadata: &CarV2BlockStore,
    metadata_forest: &mut Rc<PrivateForest>,
    content_forest: &Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<TemporalKey> {
    // Empty all roots first
    metadata.empty_roots();
    // Store the dir, then the forest, then the manifest and key
    let temporal_key = store_dir(metadata, metadata_forest, root_dir).await?;
    store_metadata_forest(metadata, metadata_forest).await?;
    store_content_forest(metadata, content_forest).await?;
    Ok(temporal_key)
}

/// Load all hot objects!
pub async fn load_all_metadata(
    key: &TemporalKey,
    metadata: &CarV2BlockStore,
) -> Result<(Rc<PrivateForest>, Rc<PrivateForest>, Rc<PrivateDirectory>)> {
    let content_forest = load_content_forest(metadata, 1).await?;
    let metadata_forest = load_metadata_forest(metadata, 1).await?;
    let dir = load_dir(metadata, key, &metadata_forest).await?;
    Ok((metadata_forest, content_forest, dir))
}

/// Store everything at once!
pub async fn store_all(
    metadata: &CarV2BlockStore,
    content: &CarV2BlockStore,
    metadata_forest: &mut Rc<PrivateForest>,
    content_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<TemporalKey> {
    // Empty all roots first
    metadata.empty_roots();
    content.empty_roots();

    let temporal_key = store_dir(metadata, metadata_forest, root_dir).await?;
    store_metadata_forest(metadata, metadata_forest).await?;

    store_content_forest(metadata, content_forest).await?;
    store_content_forest(content, content_forest).await?;

    Ok(temporal_key)
}

/// Load everything at once!
pub async fn load_all(
    key: &TemporalKey,
    metadata: &CarV2BlockStore,
    content: &CarV2BlockStore,
) -> Result<(Rc<PrivateForest>, Rc<PrivateForest>, Rc<PrivateDirectory>)> {
    let metadata_forest = load_metadata_forest(metadata, 1).await?;
    let content_forest = if let Ok(forest) = load_content_forest(metadata, 2).await {
        forest
    } else {
        load_content_forest(content, 0).await?
    };
    let dir = load_dir(metadata, key, &metadata_forest).await?;
    Ok((metadata_forest, content_forest, dir))
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use chrono::Utc;
    use serial_test::serial;
    use wnfs::libipld::IpldCodec;

    use crate::utils::{serialize::*, tests::*};

    #[tokio::test]
    #[serial]
    async fn serial_metadata_forest() -> Result<()> {
        let test_name = "serial_metadata_forest";
        // Start er up!
        let (_, _, config, metadata_forest, _, _) = &mut setup(test_name).await?;

        // Store and load
        store_metadata_forest(&config.metadata, metadata_forest).await?;
        let new_metadata_forest = &mut load_metadata_forest(&config.metadata, 0).await?;

        // Assert equality
        assert_eq!(
            new_metadata_forest
                .diff(metadata_forest, &config.metadata)
                .await?
                .len(),
            0
        );

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn serial_content_forest() -> Result<()> {
        let test_name = "serial_content_forest";
        // Start er up!
        let (_, _, config, _, content_forest, _) = &mut setup(test_name).await?;

        // Store and load
        store_content_forest(&config.content, content_forest).await?;
        let new_content_forest = &mut load_content_forest(&config.content, 0).await?;

        // Assert equality
        assert_eq!(
            new_content_forest
                .diff(content_forest, &config.content)
                .await?
                .len(),
            0
        );

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn serial_dir_object() -> Result<()> {
        let test_name = "serial_dir_local";
        // Start er up!
        let (_, _, config, metadata_forest, _, dir) = &mut setup(test_name).await?;

        let key = &store_dir(&config.metadata, metadata_forest, dir).await?;
        store_metadata_forest(&config.metadata, metadata_forest).await?;
        let new_metadata_forest = &load_metadata_forest(&config.metadata, 1).await?;
        let new_dir = &mut load_dir(&config.metadata, key, new_metadata_forest).await?;
        // Assert equality
        assert_eq!(dir, new_dir);
        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn serial_dir_content() -> Result<()> {
        let test_name = "serial_dir_content";
        // Start er up!
        let (_, _, config, original_metadata_forest, original_content_forest, original_dir) =
            &mut setup(test_name).await?;

        // Grab the original file
        let original_file = original_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                original_metadata_forest,
                &config.metadata,
                &mut thread_rng(),
            )
            .await?;

        // Get the content
        let original_content = original_file
            .get_content(original_content_forest, &config.content)
            .await?;

        let key = &store_dir(&config.metadata, original_metadata_forest, original_dir).await?;
        store_metadata_forest(&config.metadata, original_metadata_forest).await?;

        let new_metadata_forest = &mut load_metadata_forest(&config.metadata, 1).await?;
        let new_dir = &mut load_dir(&config.metadata, key, new_metadata_forest).await?;
        // Assert equality
        assert_eq!(original_dir, new_dir);

        let file = new_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                new_metadata_forest,
                &config.metadata,
                &mut thread_rng(),
            )
            .await?;
        // Get the content
        let new_content = file
            .get_content(original_content_forest, &config.content)
            .await?;

        assert_eq!(original_content, new_content);

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn serial_all_metadata() -> Result<()> {
        let test_name = "serial_all_metadata";
        // Start er up!
        let (_, _, config, metadata_forest, content_forest, dir) = &mut setup(test_name).await?;

        let key =
            &store_all_metadata(&config.metadata, metadata_forest, content_forest, dir).await?;

        let (new_metadata_forest, _, new_dir) =
            &mut load_all_metadata(key, &config.metadata).await?;

        // Assert equality
        assert_eq!(
            new_metadata_forest
                .diff(metadata_forest, &config.metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(dir, new_dir);
        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn serial_all() -> Result<()> {
        let test_name = "serial_all";
        // Start er up!
        let (_, _, config, metadata_forest, content_forest, dir) = &mut setup(test_name).await?;

        let key = &store_all(
            &config.metadata,
            &config.content,
            metadata_forest,
            content_forest,
            dir,
        )
        .await?;

        let (new_metadata_forest, new_content_forest, new_dir) =
            &mut load_all(key, &config.metadata, &config.content).await?;

        // Assert equality
        assert_eq!(
            new_metadata_forest
                .diff(metadata_forest, &config.metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(
            new_content_forest
                .diff(content_forest, &config.content)
                .await?
                .len(),
            0
        );
        assert_eq!(dir, new_dir);
        // Teardown
        teardown(test_name).await
    }
}
