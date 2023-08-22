use anyhow::Result;
use rand::thread_rng;
use std::{collections::BTreeMap, rc::Rc};
use tomb_crypt::prelude::*;
use wnfs::{
    common::{dagcbor, AsyncSerialize, BlockStore as WnfsBlockStore, HashOutput},
    libipld::{serde as ipld_serde, Cid, Ipld, IpldCodec},
    private::{
        PrivateDirectory, PrivateForest, PrivateNode, PrivateNodeOnPathHistory, PrivateRef,
        TemporalKey,
    },
};

use crate::{
    types::{blockstore::tombblockstore::TombBlockStore, keys::manager::Manager},
    utils::error::SerialError,
};

/// Store a given PrivateForest in a given Store
async fn store_forest(
    forest: &Rc<PrivateForest>,
    serializer: &impl TombBlockStore,
    storage: &impl TombBlockStore,
) -> Result<Cid> {
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(serializer).await?;
    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = storage.put_serializable(&forest_ipld).await?;
    // Return Ok
    Ok(ipld_cid)
}

/// Load a given PrivateForest from a given Store
async fn load_forest(cid: &Cid, store: &impl TombBlockStore) -> Result<Rc<PrivateForest>> {
    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = store.get_deserializable(cid).await?;
    // Create a PrivateForest from that IPLD DAG
    let forest: Rc<PrivateForest> = Rc::new(
        ipld_serde::from_ipld::<PrivateForest>(forest_ipld)
            .expect("failed to convert IPLD to PrivateForest"),
    );
    // Return
    Ok(forest)
}

/// Store a PrivateDirectory
async fn store_dir<M: TombBlockStore>(
    store: &M,
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
async fn load_dir<M: TombBlockStore>(
    store: &M,
    temporal_key: &TemporalKey,
    ref_cid: &Cid,
    metadata_forest: &Rc<PrivateForest>,
) -> Result<Rc<PrivateDirectory>> {
    // Construct the saturated name hash
    let (saturated_name_hash, content_cid): (HashOutput, Cid) = store
        .get_deserializable::<(HashOutput, Cid)>(ref_cid)
        .await?;

    // Reconstruct the PrivateRef
    let dir_ref: PrivateRef =
        PrivateRef::with_temporal_key(saturated_name_hash, temporal_key.clone(), content_cid);

    // Load the PrivateDirectory from the PrivateForest
    PrivateNode::load(&dir_ref, metadata_forest, store)
        .await?
        .as_dir()
}

/// Grabs the cid of the original PrivateRef
pub async fn get_original_ref_cid<M: TombBlockStore>(store: &M) -> Result<Cid> {
    // If we can successfully extract the Cid
    if let Some(root) = store.get_root() &&
       let Ok(Ipld::Map(metadata_map)) = store.get_deserializable::<Ipld>(&root).await &&
       let Some(Ipld::Link(original_ref_cid)) = metadata_map.get("original_ref") {
        // Return it
        Ok(*original_ref_cid)
    } else {
        Err(SerialError::MissingMetadata("original ref_cid".to_string()).into())
    }
}

/// Store both dirs in both BlockStores, update keys
pub async fn store_dirs_update_keys<M: TombBlockStore, C: TombBlockStore>(
    metadata: &M,
    content: &C,
    metadata_forest: &mut Rc<PrivateForest>,
    content_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
    manager: &mut Manager,
) -> Result<(Cid, Cid)> {
    // Store PrivateDirectory in both BlockStores, retrieving the new TemporalKey and cid of remaining PrivateRef components
    let (ref_cid1, temporal_key1) = store_dir(metadata, metadata_forest, root_dir).await?;
    let (ref_cid2, temporal_key2) = store_dir(content, content_forest, root_dir).await?;
    assert_eq!(ref_cid1, ref_cid2);
    assert_eq!(temporal_key1, temporal_key2);

    // Update the temporal key in the key manager
    manager.update_current_key(&temporal_key1).await?;
    // If we've yet to initialize our originals
    let original_ref_cid = if metadata.get_root().expect("failed to get root") == Cid::default() {
        // Set the original key
        manager.set_original_key(&temporal_key1).await?;
        // Return
        ref_cid1
    } else {
        // Grab
        get_original_ref_cid(metadata).await?
    };

    // Return the private ref CIDs
    Ok((original_ref_cid, ref_cid1))
}

/// Stpre both forests in both BlockStores
pub async fn store_forests<M: TombBlockStore, C: TombBlockStore>(
    metadata: &M,
    content: &C,
    metadata_forest: &Rc<PrivateForest>,
    content_forest: &Rc<PrivateForest>,
) -> Result<(Cid, Cid)> {
    // Store the metadata PrivateForest in both the content and the metadata BlockStores
    let metadata_forest_cid1 = store_forest(metadata_forest, metadata, metadata).await?;
    let metadata_forest_cid2 = store_forest(metadata_forest, metadata, content).await?;
    assert_eq!(metadata_forest_cid1, metadata_forest_cid2);

    // Store the content PrivateForest in both the content and the metadata BlockStores
    let content_forest_cid1 = store_forest(content_forest, content, content).await?;
    let content_forest_cid2 = store_forest(content_forest, content, metadata).await?;
    assert_eq!(content_forest_cid1, content_forest_cid2);
    // Ok
    Ok((metadata_forest_cid1, content_forest_cid1))
}

/// Store all relevant metadata in both BlockStores
pub async fn store_all<M: TombBlockStore, C: TombBlockStore>(
    metadata: &M,
    content: &C,
    metadata_forest: &mut Rc<PrivateForest>,
    content_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
    manager: &mut Manager,
    _manager_cid: &Cid,
) -> Result<()> {
    // Store dirs, update keys
    let (original_ref_cid, current_ref_cid) = store_dirs_update_keys(
        metadata,
        content,
        metadata_forest,
        content_forest,
        root_dir,
        manager,
    )
    .await?;

    // Store forests
    let (metadata_forest_cid, content_forest_cid) =
        store_forests(metadata, content, metadata_forest, content_forest).await?;

    // TODO only update content for Key Manager
    // let manager_cid = update_manager(manager, manager_cid, metadata, content).await?;
    let manager_cid = store_manager(manager, metadata, content).await?;

    // Store everything
    store_ipld(
        metadata,
        content,
        original_ref_cid,
        current_ref_cid,
        metadata_forest_cid,
        content_forest_cid,
        manager_cid,
    )
    .await
}

/// Store everything at once!
pub async fn store_ipld<M: TombBlockStore, C: TombBlockStore>(
    metadata: &M,
    content: &C,
    original_ref_cid: Cid,
    current_ref_cid: Cid,
    metadata_forest_cid: Cid,
    content_forest_cid: Cid,
    manager_cid: Cid,
) -> Result<()> {
    // Construct new map for metadata
    let mut metadata_map = BTreeMap::new();
    // Set all key values
    metadata_map.insert("original_ref".to_string(), Ipld::Link(original_ref_cid));
    metadata_map.insert("current_ref".to_string(), Ipld::Link(current_ref_cid));
    metadata_map.insert("manager".to_string(), Ipld::Link(manager_cid));
    metadata_map.insert(
        "metadata_forest".to_string(),
        Ipld::Link(metadata_forest_cid),
    );
    metadata_map.insert("content_forest".to_string(), Ipld::Link(content_forest_cid));
    // Build features
    metadata_map.insert(
        "build_features".to_string(),
        Ipld::String(env!("BUILD_FEATURES").to_string()),
    );
    metadata_map.insert(
        "build_profile".to_string(),
        Ipld::String(env!("BUILD_PROFILE").to_string()),
    );
    metadata_map.insert(
        "repo_version".to_string(),
        Ipld::String(env!("REPO_VERSION").to_string()),
    );
    // Now that we've finished inserting
    let metadata_root = &Ipld::Map(metadata_map);
    // Put the metadata IPLD Map into BlockStores
    let metadata_root_cid = metadata.put_serializable(metadata_root).await?;
    let content_root_cid = content.put_serializable(metadata_root).await?;
    // Set roots
    metadata.set_root(&metadata_root_cid);
    content.set_root(&content_root_cid);
    // Ok
    Ok(())
}

/// Load both PrivateForests from metadata
pub async fn load_forests<M: TombBlockStore>(
    metadata: &M,
    root: &BTreeMap<String, Ipld>,
) -> Result<(Rc<PrivateForest>, Rc<PrivateForest>)> {
    if let Some(Ipld::Link(metadata_forest_cid)) = root.get("metadata_forest") &&
    let Some(Ipld::Link(content_forest_cid)) = root.get("content_forest") {
        let metadata_forest = load_forest(metadata_forest_cid, metadata).await?;
        let content_forest = load_forest(content_forest_cid, metadata).await?;
        Ok((metadata_forest, content_forest))
    }
    else {
        Err(SerialError::MissingMetadata("forests".to_string()).into())
    }
}

/// Load everything at once!
pub async fn load_all<M: TombBlockStore>(
    wrapping_key: &EcEncryptionKey,
    metadata: &M,
) -> Result<(
    Rc<PrivateForest>,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
    Manager,
    Cid,
)> {
    // Load the IPLD map
    if let Some(metadata_root) = metadata.get_root() &&
        let Ok(Ipld::Map(root)) = metadata.get_deserializable::<Ipld>(&metadata_root).await &&
        let (metadata_forest, content_forest) = load_forests(metadata, &root).await? &&
        let Some(Ipld::Link(current_ref_cid)) = root.get("current_ref") &&
        let Some(Ipld::Link(manager_cid)) = root.get("manager") {
        // Load in the objects        
        let mut manager = metadata.get_deserializable::<Manager>(manager_cid).await?;
        // Load in the Temporal Keys to memory
        manager.load_temporal_keys(wrapping_key).await?;
        let current_key = &manager.retrieve_current(wrapping_key).await?;
        let current_directory = load_dir(metadata, current_key, current_ref_cid, &metadata_forest).await?;
        // Return Ok with loaded objectsd
        Ok((metadata_forest, content_forest, current_directory, manager, *manager_cid))
    }
    else {
        Err(SerialError::MissingMetadata("IPLD Map".to_string()).into())
    }
}

/// Obtain a PrivateNodeOnPathHistory iterator for the root directory
pub async fn load_history<M: TombBlockStore>(
    wrapping_key: &EcEncryptionKey,
    metadata: &M,
) -> Result<PrivateNodeOnPathHistory> {
    let (metadata_forest, _, current_directory, manager, _) =
        load_all(wrapping_key, metadata).await?;

    // Grab the original key
    let original_key = &manager.retrieve_original(wrapping_key).await?;
    // Load the original PrivateRef cid
    let original_ref_cid = &get_original_ref_cid(metadata).await?;
    // Load dir
    let original_directory =
        load_dir(metadata, original_key, original_ref_cid, &metadata_forest).await?;

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

/// Extract the details of the build that created this metadata
pub async fn load_build_details<M: TombBlockStore>(
    metadata: &M,
) -> Result<(String, String, String)> {
    // Load the IPLD map
    if let Some(metadata_root) = metadata.get_root() &&
        let Ok(Ipld::Map(root)) = metadata.get_deserializable::<Ipld>(&metadata_root).await &&
        let Some(Ipld::String(build_features)) = root.get("build_features") &&
        let Some(Ipld::String(build_profile)) = root.get("build_profile") &&
        let Some(Ipld::String(repo_version)) = root.get("repo_version") {
        // Ok
        Ok((build_features.to_string(), build_profile.to_string(), repo_version.to_string()))
    } else {
        Err(SerialError::MissingMetadata("build details lost".to_string()).into())
    }
}

/// Store the key Manager in both BlockStores
pub async fn store_manager(
    manager: &Manager,
    metadata: &impl WnfsBlockStore,
    content: &impl WnfsBlockStore,
) -> Result<Cid> {
    let bytes = dagcbor::encode(manager)?;
    let cid1 = metadata
        .put_block(bytes.clone(), IpldCodec::DagCbor)
        .await?;
    let cid2 = content.put_block(bytes, IpldCodec::DagCbor).await?;
    assert_eq!(cid1, cid2);
    Ok(cid1)
}

/// Update the content of the Managers in place within both BlockStores
pub async fn update_manager(
    manager: &Manager,
    manager_cid: &Cid,
    metadata: &impl TombBlockStore,
    content: &impl TombBlockStore,
) -> Result<Cid> {
    let bytes = dagcbor::encode(&manager)?;
    // Update content in place
    let cid1 = metadata
        .update_block(manager_cid, bytes.clone(), IpldCodec::DagCbor)
        .await?;
    let cid2 = content
        .update_block(manager_cid, bytes, IpldCodec::DagCbor)
        .await?;
    assert_eq!(cid1, cid2);
    Ok(cid1)
}

#[cfg(test)]
mod test {
    use crate::utils::{serialize::*, test::*};
    use anyhow::Result;
    use chrono::Utc;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn forest() -> Result<()> {
        let test_name = "forest";
        // Start er up!
        let (metadata, _, metadata_forest, _, _) = &mut setup_memory(test_name).await?;

        // Store and load
        let metadata_forest_cid = store_forest(metadata_forest, metadata, metadata).await?;
        let new_metadata_forest = &mut load_forest(&metadata_forest_cid, metadata).await?;

        // Assert equality
        assert_eq!(
            new_metadata_forest
                .diff(metadata_forest, metadata)
                .await?
                .len(),
            0
        );

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn dir_object() -> Result<()> {
        let test_name = "dir_object";
        // Start er up!
        let (metadata, _, metadata_forest, _, dir) = &mut setup_memory(test_name).await?;

        let (private_ref_cid, temporal_key) = &store_dir(metadata, metadata_forest, dir).await?;
        let metadata_forest_cid = store_forest(metadata_forest, metadata, metadata).await?;
        let new_metadata_forest = &load_forest(&metadata_forest_cid, metadata).await?;
        let new_dir =
            &mut load_dir(metadata, temporal_key, private_ref_cid, new_metadata_forest).await?;
        // Assert equality
        assert_eq!(dir, new_dir);
        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn dir_content() -> Result<()> {
        let test_name = "dir_content";
        // Start er up!
        let (metadata, content, original_metadata_forest, original_content_forest, original_dir) =
            &mut setup_memory(test_name).await?;

        // Grab the original file
        let original_file = original_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                original_metadata_forest,
                metadata,
                &mut thread_rng(),
            )
            .await?;

        // Get the content
        let original_content = original_file
            .get_content(original_content_forest, content)
            .await?;

        let (private_ref_cid, temporal_key) =
            &store_dir(metadata, original_metadata_forest, original_dir).await?;
        let metadata_forest_cid =
            store_forest(original_metadata_forest, metadata, metadata).await?;

        let new_metadata_forest = &mut load_forest(&metadata_forest_cid, metadata).await?;
        let new_dir =
            &mut load_dir(metadata, temporal_key, private_ref_cid, new_metadata_forest).await?;
        // Assert equality
        assert_eq!(original_dir, new_dir);

        let file = new_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                new_metadata_forest,
                metadata,
                &mut thread_rng(),
            )
            .await?;
        // Get the content
        let new_content = file.get_content(original_content_forest, content).await?;

        assert_eq!(original_content, new_content);

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn all_from_metadata() -> Result<()> {
        let test_name = "all";
        // Start er up!
        let (metadata, content, metadata_forest, content_forest, dir) =
            &mut setup_memory(test_name).await?;
        let wrapping_key = EcEncryptionKey::generate().await?;
        let manager = &mut Manager::default();
        manager.insert(&wrapping_key.public_key()?).await?;
        let manager_cid = &store_manager(manager, metadata, content).await?;

        let _ = &store_all(
            metadata,
            content,
            metadata_forest,
            content_forest,
            dir,
            manager,
            manager_cid,
        )
        .await?;

        let (new_metadata_forest, new_content_forest, new_dir, new_manager, _) =
            &mut load_all(&wrapping_key, metadata).await?;

        // Assert equality
        assert_eq!(
            new_metadata_forest
                .diff(metadata_forest, metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(
            new_content_forest
                .diff(content_forest, content)
                .await?
                .len(),
            0
        );
        assert_eq!(dir, new_dir);
        assert_eq!(manager, new_manager);
        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn all_from_content() -> Result<()> {
        let test_name = "all";
        // Start er up!
        let (metadata, content, metadata_forest, content_forest, dir) =
            &mut setup_memory(test_name).await?;
        let wrapping_key = EcEncryptionKey::generate().await?;
        let manager = &mut Manager::default();
        manager.insert(&wrapping_key.public_key()?).await?;

        let manager_cid = &store_manager(manager, metadata, content).await?;
        let _ = &store_all(
            metadata,
            content,
            metadata_forest,
            content_forest,
            dir,
            manager,
            manager_cid,
        )
        .await?;

        let (new_metadata_forest, new_content_forest, new_dir, new_manager, _) =
            &mut load_all(&wrapping_key, content).await?;

        // Assert equality
        assert_eq!(
            new_metadata_forest
                .diff(metadata_forest, metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(
            new_content_forest
                .diff(content_forest, content)
                .await?
                .len(),
            0
        );
        assert_eq!(dir, new_dir);
        assert_eq!(manager, new_manager);
        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn history() -> Result<()> {
        let test_name = "history";
        // Start er up!
        let (metadata, content, metadata_forest, content_forest, dir) =
            &mut setup_memory(test_name).await?;
        let wrapping_key = EcEncryptionKey::generate().await?;
        let manager = &mut Manager::default();
        manager.insert(&wrapping_key.public_key()?).await?;
        let manager_cid = &store_manager(manager, metadata, content).await?;

        // Store everything
        let _ = &store_all(
            metadata,
            content,
            metadata_forest,
            content_forest,
            dir,
            manager,
            manager_cid,
        )
        .await?;

        let _history = load_history(&wrapping_key, metadata).await?;

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn build_details() -> Result<()> {
        let test_name = "build_details";
        // Start er up!
        let (metadata, content, metadata_forest, content_forest, dir) =
            &mut setup_memory(test_name).await?;
        let wrapping_key = EcEncryptionKey::generate().await?;
        let manager = &mut Manager::default();
        manager.insert(&wrapping_key.public_key()?).await?;
        let manager_cid = &store_manager(manager, metadata, content).await?;

        // Store everything
        let _ = &store_all(
            metadata,
            content,
            metadata_forest,
            content_forest,
            dir,
            manager,
            manager_cid,
        )
        .await?;

        // Assert we can successfully load them
        assert!(load_build_details(metadata).await.is_ok());

        // Teardown
        teardown(test_name).await
    }
}
