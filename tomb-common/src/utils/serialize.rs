use crate::{
    crypto::rsa::RsaPrivateKey,
    types::{
        blockstore::car::carv2::blockstore::BlockStore,
        config::{error::ConfigError, keys::manager::Manager},
    },
};
use anyhow::Result;
use rand::thread_rng;
use std::{collections::BTreeMap, rc::Rc};
use wnfs::{
    common::{AsyncSerialize, BlockStore as WnfsBlockStore, HashOutput},
    libipld::{serde as ipld_serde, Cid, Ipld},
    private::{
        PrivateDirectory, PrivateForest, PrivateNode, PrivateNodeOnPathHistory, PrivateRef,
        TemporalKey,
    },
};

/// Store a given PrivateForest in a given Store
async fn store_forest(forest: &Rc<PrivateForest>, store: &impl WnfsBlockStore) -> Result<Cid> {
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(store).await?;
    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = store.put_serializable(&forest_ipld).await?;
    // Return Ok
    Ok(ipld_cid)
}

/// Load a given PrivateForest from a given Store
async fn load_forest(cid: &Cid, store: &impl WnfsBlockStore) -> Result<Rc<PrivateForest>> {
    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = store.get_deserializable(cid).await?;
    // Create a PrivateForest from that IPLD DAG
    let forest: Rc<PrivateForest> =
        Rc::new(ipld_serde::from_ipld::<PrivateForest>(forest_ipld).unwrap());
    // Return
    Ok(forest)
}

/// Store a PrivateDirectory
async fn store_dir(
    store: &BlockStore,
    metadata_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<(Cid, TemporalKey)> {
    // Random number generator
    let rng = &mut thread_rng();

    // Store the root of the PrivateDirectory in the PrivateForest, retrieving a PrivateRef to it
    let dir_ref: PrivateRef = root_dir.store(metadata_forest, store, rng).await?;

    // Extract the component fields of the PrivateDirectory's PrivateReference
    let PrivateRef {
        saturated_name_hash,
        temporal_key,
        content_cid,
    } = dir_ref;

    // Store it in the Metadata BlockStore
    let ref_cid = store
        .put_serializable::<(HashOutput, Cid)>(&(saturated_name_hash, content_cid))
        .await?;

    // Return OK
    Ok((ref_cid, temporal_key))
}

/// Load a PrivateDirectory
async fn load_dir(
    store: &BlockStore,
    temporal_key: &TemporalKey,
    private_ref_cid: &Cid,
    metadata_forest: &Rc<PrivateForest>,
) -> Result<Rc<PrivateDirectory>> {
    // Construct the saturated name hash
    let (saturated_name_hash, content_cid): (HashOutput, Cid) = store
        .get_deserializable::<(HashOutput, Cid)>(private_ref_cid)
        .await?;

    // Reconstruct the PrivateRef
    let dir_ref: PrivateRef =
        PrivateRef::with_temporal_key(saturated_name_hash, temporal_key.clone(), content_cid);

    // Load the PrivateDirectory from the PrivateForest
    PrivateNode::load(&dir_ref, metadata_forest, store)
        .await?
        .as_dir()
}

/// Store everything at once!
pub async fn store_all(
    metadata: &BlockStore,
    content: &BlockStore,
    metadata_forest: &mut Rc<PrivateForest>,
    content_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
    key_manager: &mut Manager,
) -> Result<()> {
    // Construct new map for metadata
    let mut metadata_map = BTreeMap::new();
    // Store PrivateDirectory in the metadata BlockStore, retrieving the new TemporalKey and cid of remaining PrivateRef components
    let (private_ref_cid, temporal_key) = store_dir(metadata, metadata_forest, root_dir).await?;
    // Update the temporal key in the key manager
    key_manager.update_current_key(&temporal_key).await?;
    metadata_map.insert(
        "current_private_ref".to_string(),
        Ipld::Link(private_ref_cid),
    );
    // If we've yet to initialize our originals
    if metadata.get_root().unwrap() == Cid::default() {
        // Set the original key
        key_manager.set_original_key(&temporal_key).await?;
        // Insert private ref and set original key in key manager
        metadata_map.insert(
            "original_private_ref".to_string(),
            Ipld::Link(private_ref_cid),
        );
    }
    // If they're already present
    else {
        // Simply ensure the cid reference is carried over by reinserting
        metadata_map.insert(
            "original_private_ref".to_string(),
            Ipld::Link(get_original_private_ref_cid(metadata).await?),
        );
    }
    // Put the key manager in both the content and the metadata BlockStores
    let key_manager_cid = metadata.put_serializable(key_manager).await?;
    metadata_map.insert("key_manager".to_string(), Ipld::Link(key_manager_cid));

    // Store the metadata PrivateForest in both the content and the metadata BlockStores
    let metadata_forest_cid = store_forest(metadata_forest, metadata).await?;
    metadata_map.insert(
        "metadata_forest".to_string(),
        Ipld::Link(metadata_forest_cid),
    );

    // Store the content PrivateForest in both the content and the metadata BlockStores
    let content_forest_cid = store_forest(content_forest, content).await?;
    metadata_map.insert("content_forest".to_string(), Ipld::Link(content_forest_cid));

    // Now that we've finished inserting
    let metadata_root = &Ipld::Map(metadata_map);
    // Put the metadata IPLD Map into the metadata BlockStore and set root
    let metadata_root_cid = metadata.put_serializable(metadata_root).await?;
    metadata.set_root(&metadata_root_cid);
    // Put the metadata IPLD Map into the content BlockStore and set root
    let content_root_cid = content.put_serializable(metadata_root).await?;
    content.set_root(&content_root_cid);

    Ok(())
}

pub async fn get_original_private_ref_cid(metadata: &BlockStore) -> Result<Cid> {
    //
    if let Some(metadata_root) = metadata.get_root() &&
       let Ok(Ipld::Map(metadata_map)) = metadata.get_deserializable::<Ipld>(&metadata_root).await &&
       let Some(Ipld::Link(original_private_ref_cid)) = metadata_map.get("original_private_ref") {
           Ok(*original_private_ref_cid)
    }
    else {
        Err(ConfigError::MissingMetadata("original private_ref_cid".to_string()).into())
    }
}

pub async fn load_history(
    wrapping_key: &RsaPrivateKey,
    metadata: &BlockStore,
    content: &BlockStore,
) -> Result<PrivateNodeOnPathHistory> {
    let (metadata_forest, _, current_directory, key_manager) =
        load_all(wrapping_key, metadata, content).await?;

    // Grab the original key
    let original_key = &key_manager.retrieve_original(wrapping_key).await?;
    // Load the original PrivateRef cid
    let original_private_ref_cid = &get_original_private_ref_cid(metadata).await?;
    // Load dir
    let original_directory = load_dir(
        metadata,
        original_key,
        original_private_ref_cid,
        &metadata_forest,
    )
    .await?;

    PrivateNodeOnPathHistory::of(
        current_directory,
        original_directory,
        1_000_000,
        &[],
        true,
        metadata_forest,
        metadata,
    )
    .await
}

/// Load everything at once!
pub async fn load_all(
    wrapping_key: &RsaPrivateKey,
    metadata: &BlockStore,
    content: &BlockStore,
) -> Result<(
    Rc<PrivateForest>,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
    Manager,
)> {
    // Load the IPLD map either from the metadata BlockStore, or the content BlockStore
    let map =
        if let Some(metadata_root) = metadata.get_root() &&
           let Ok(Ipld::Map(map)) = metadata.get_deserializable::<Ipld>(&metadata_root).await {
            map
        }
        else if let Some(content_root) = content.get_root() &&
                let Ok(Ipld::Map(map)) = content.get_deserializable::<Ipld>(&content_root).await {
            map
        } else {
            return Err(ConfigError::MissingMetadata("IPLD Map".to_string()).into())
        };

    // If we are able to find all CIDs
    if let Some(Ipld::Link(metadata_forest_cid)) = map.get("metadata_forest") &&
    let Some(Ipld::Link(current_private_ref_cid)) = map.get("current_private_ref") &&
    let Some(Ipld::Link(key_manager_cid)) = map.get("key_manager") &&
    let Some(Ipld::Link(content_forest_cid)) = map.get("content_forest")
    {
        // Load in the objects
        let metadata_forest = load_forest(metadata_forest_cid, metadata).await?;
        let content_forest = load_forest(content_forest_cid, content).await?;
        let mut key_manager = metadata.get_deserializable::<Manager>(key_manager_cid).await?;
        // Load in the Temporal Keys to memory
        key_manager.load_temporal_keys(wrapping_key).await?;
        let current_key = &key_manager.retrieve_current(wrapping_key).await?;
        let current_directory = load_dir(metadata, current_key, current_private_ref_cid, &metadata_forest).await?;
        // Return Ok with loaded objectsd
        Ok((metadata_forest, content_forest, current_directory, key_manager))
    }
    else {
        Err(ConfigError::MissingMetadata("One or both BlockStores are missing CIDs".to_string()).into())
    }
}

#[cfg(test)]
mod test {
    use crate::utils::{serialize::*, tests::*};
    use anyhow::Result;
    use chrono::Utc;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn serial_forest() -> Result<()> {
        let test_name = "serial_metadata_forest";
        // Start er up!
        let (_, _, config, metadata_forest, _, _) = &mut setup(test_name).await?;

        // Store and load
        let metadata_forest_cid = store_forest(metadata_forest, &config.metadata).await?;
        let new_metadata_forest = &mut load_forest(&metadata_forest_cid, &config.metadata).await?;

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
    async fn serial_dir_object() -> Result<()> {
        let test_name = "serial_dir_object";
        // Start er up!
        let (_, _, config, metadata_forest, _, dir) = &mut setup(test_name).await?;

        let (private_ref_cid, temporal_key) =
            &store_dir(&config.metadata, metadata_forest, dir).await?;
        let metadata_forest_cid = store_forest(metadata_forest, &config.metadata).await?;
        let new_metadata_forest = &load_forest(&metadata_forest_cid, &config.metadata).await?;
        let new_dir = &mut load_dir(
            &config.metadata,
            temporal_key,
            private_ref_cid,
            new_metadata_forest,
        )
        .await?;
        // Assert equality
        assert_eq!(dir, new_dir);
        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
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

        let (private_ref_cid, temporal_key) =
            &store_dir(&config.metadata, original_metadata_forest, original_dir).await?;
        let metadata_forest_cid = store_forest(original_metadata_forest, &config.metadata).await?;

        let new_metadata_forest = &mut load_forest(&metadata_forest_cid, &config.metadata).await?;
        let new_dir = &mut load_dir(
            &config.metadata,
            temporal_key,
            private_ref_cid,
            new_metadata_forest,
        )
        .await?;
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
    async fn serial_all() -> Result<()> {
        let test_name = "serial_all";
        // Start er up!
        let (_, global, config, metadata_forest, content_forest, dir) =
            &mut setup(test_name).await?;
        let wrapping_key = global.wrapping_key_from_disk()?;
        let mut key_manager = Manager::default();
        key_manager.insert(&wrapping_key.get_public_key()).await?;

        let _ = &store_all(
            &config.metadata,
            &config.content,
            metadata_forest,
            content_forest,
            dir,
            &mut key_manager,
        )
        .await?;

        let (new_metadata_forest, new_content_forest, new_dir, new_key_manager) =
            &mut load_all(&wrapping_key, &config.metadata, &config.content).await?;

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
        assert_eq!(&mut key_manager, new_key_manager);
        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn serial_history() -> Result<()> {
        let test_name = "serial_all";
        // Start er up!
        let (_, global, config, metadata_forest, content_forest, dir) =
            &mut setup(test_name).await?;

        let wrapping_key = global.wrapping_key_from_disk()?;
        let mut key_manager = Manager::default();
        key_manager.insert(&wrapping_key.get_public_key()).await?;

        // Store everything
        let _ = &store_all(
            &config.metadata,
            &config.content,
            metadata_forest,
            content_forest,
            dir,
            &mut key_manager,
        )
        .await?;

        let _history = load_history(&wrapping_key, &config.metadata, &config.content).await?;

        // Teardown
        teardown(test_name).await
    }
}
