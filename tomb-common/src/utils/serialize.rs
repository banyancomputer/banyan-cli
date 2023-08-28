use crate::share::manager::ShareManager;
use anyhow::Result;
use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};
use std::rc::Rc;
use wnfs::{
    common::{dagcbor, AsyncSerialize, BlockStore},
    libipld::{serde as ipld_serde, Cid, Ipld, IpldCodec},
    private::{PrivateDirectory, PrivateForest, PrivateNode, PrivateRef},
};

/// Store a given PrivateDirectory in a given Store
pub async fn store_dir<MBS: BlockStore, CBS: BlockStore>(
    metadata_store: &MBS,
    content_store: &CBS,
    metadata_forest: &mut Rc<PrivateForest>,
    content_forest: &mut Rc<PrivateForest>,
    dir: &Rc<PrivateDirectory>,
) -> Result<PrivateRef> {
    // Get a seeded source of randomness
    let seed = thread_rng().gen::<[u8; 32]>();
    let mut rng = StdRng::from_seed(seed);
    // Store the PrivateDirectory in both PrivateForests
    let metadata_ref = dir.store(metadata_forest, metadata_store, &mut rng).await?;
    let content_ref = dir.store(content_forest, content_store, &mut rng).await?;
    // Assert that the PrivateRefs are the same
    assert_eq!(metadata_ref, content_ref);
    // Return Ok
    Ok(metadata_ref)
}

/// Store a given PrivateForest in a given Store
pub async fn store_forest<SBS: BlockStore, BS: BlockStore>(
    forest: &Rc<PrivateForest>,
    serializer: &SBS,
    storage: &BS,
) -> Result<Cid> {
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(serializer).await?;
    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = storage.put_serializable(&forest_ipld).await?;
    // Return Ok
    Ok(ipld_cid)
}

/// Store the key Manager in both BlockStores
pub async fn store_share_manager(
    share_manager: &ShareManager,
    store: &impl BlockStore,
) -> Result<Cid> {
    let share_manager_bytes = dagcbor::encode(share_manager)?;
    let share_manager_cid = store
        .put_block(share_manager_bytes.clone(), IpldCodec::DagCbor)
        .await?;
    Ok(share_manager_cid)
}

/// Load a given PrivateForest from a given Store
pub async fn load_forest<BS: BlockStore>(cid: &Cid, store: &BS) -> Result<Rc<PrivateForest>> {
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

/// Load a PrivateDirectory
pub async fn load_dir<BS: BlockStore>(
    store: &BS,
    private_ref: &PrivateRef,
    metadata_forest: &Rc<PrivateForest>,
) -> Result<Rc<PrivateDirectory>> {
    // Load the PrivateDirectory from the PrivateForest
    PrivateNode::load(private_ref, metadata_forest, store)
        .await?
        .as_dir()
}

#[cfg(test)]
mod test {
    use crate::utils::{serialize::*, tests::*};
    use anyhow::Result;
    use chrono::Utc;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn forest() -> Result<()> {
        let test_name = "forest";
        // Start er up!
        let (metadata, _, metadata_forest, _, _) = &mut setup_memory_test(test_name).await?;

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
        teardown_test(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn dir_object() -> Result<()> {
        let test_name = "dir_object";
        // Start er up!
        let (metadata, content, metadata_forest, content_forest, dir) =
            &mut setup_memory_test(test_name).await?;

        let private_ref =
            &store_dir(metadata, content, metadata_forest, content_forest, dir).await?;
        let metadata_forest_cid = store_forest(metadata_forest, metadata, metadata).await?;
        let new_metadata_forest = &load_forest(&metadata_forest_cid, metadata).await?;
        let new_dir = &mut load_dir(metadata, private_ref, new_metadata_forest).await?;
        // Assert equality
        assert_eq!(dir, new_dir);
        // Teardown
        teardown_test(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn dir_content() -> Result<()> {
        let test_name = "dir_content";
        // Start er up!
        let (metadata, content, original_metadata_forest, original_content_forest, original_dir) =
            &mut setup_memory_test(test_name).await?;

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
        let private_ref = &store_dir(
            metadata,
            content,
            original_metadata_forest,
            original_content_forest,
            original_dir,
        )
        .await?;
        let metadata_forest_cid =
            store_forest(original_metadata_forest, metadata, metadata).await?;

        let new_metadata_forest = &mut load_forest(&metadata_forest_cid, metadata).await?;
        let new_dir = &mut load_dir(metadata, private_ref, new_metadata_forest).await?;
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
        teardown_test(test_name).await
    }
}
