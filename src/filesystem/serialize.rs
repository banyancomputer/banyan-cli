use crate::filesystem::sharing::manager::ShareManager;
use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};
use std::rc::Rc;
use wnfs::{
    common::{dagcbor, AsyncSerialize, BlockStore},
    libipld::{serde as ipld_serde, Cid, Ipld, IpldCodec},
    private::{PrivateDirectory, PrivateForest, PrivateNode, PrivateRef},
};

use super::{error::FilesystemError, sharing::SharingError};

/// Store a given PrivateDirectory in a given Store
pub async fn store_dir<MBS: BlockStore, CBS: BlockStore>(
    metadata_store: &MBS,
    content_store: &CBS,
    forest: &mut Rc<PrivateForest>,
    dir: &Rc<PrivateDirectory>,
) -> Result<PrivateRef, FilesystemError> {
    // Get a seeded source of randomness
    let seed = thread_rng().gen::<[u8; 32]>();
    let mut rng = StdRng::from_seed(seed);
    // Store the PrivateDirectory in both PrivateForests
    let metadata_ref = dir
        .store(forest, metadata_store, &mut rng)
        .await
        .map_err(Box::from)?;
    let content_ref = dir
        .store(forest, content_store, &mut rng)
        .await
        .map_err(Box::from)?;
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
) -> Result<Cid, FilesystemError> {
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest
        .async_serialize_ipld(serializer)
        .await
        .map_err(|err| FilesystemError::sharing(SharingError::invalid_data(&err.to_string())))?;
    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = storage
        .put_serializable(&forest_ipld)
        .await
        .map_err(Box::from)?;
    // Return Ok
    Ok(ipld_cid)
}

/// Store the key Manager in both BlockStores
pub async fn store_share_manager(
    share_manager: &ShareManager,
    store: &impl BlockStore,
) -> Result<Cid, FilesystemError> {
    let share_manager_bytes = dagcbor::encode(share_manager).map_err(Box::from)?;
    let share_manager_cid = store
        .put_block(share_manager_bytes.clone(), IpldCodec::DagCbor)
        .await
        .map_err(Box::from)?;
    Ok(share_manager_cid)
}

/// Load a given PrivateForest from a given Store
pub async fn load_forest<BS: BlockStore>(
    cid: &Cid,
    store: &BS,
) -> Result<Rc<PrivateForest>, FilesystemError> {
    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = store.get_deserializable(cid).await.map_err(Box::from)?;
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
    forest: &Rc<PrivateForest>,
) -> Result<Rc<PrivateDirectory>, FilesystemError> {
    // Load the PrivateDirectory from the PrivateForest
    PrivateNode::load(private_ref, forest, store)
        .await
        .map_err(Box::from)?
        .as_dir()
        .map_err(Box::from)
        .map_err(FilesystemError::wnfs)
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
    use crate::{
        filesystem::serialize::{load_dir, load_forest, store_dir, store_forest},
        utils::{
            testing::blockstores::{setup_memory_test, teardown_test},
            UtilityError,
        },
    };
    use chrono::Utc;
    use rand::thread_rng;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn forest() -> Result<(), UtilityError> {
        let test_name = "forest";
        // Start er up!
        let (metadata, _, forest, _) = &mut setup_memory_test(test_name).await?;

        // Store and load
        let forest_cid = store_forest(forest, metadata, metadata).await?;
        let new_forest = load_forest(&forest_cid, metadata).await?;

        // Assert equality
        assert_eq!(
            new_forest
                .diff(forest, metadata)
                .await
                .map_err(Box::from)?
                .len(),
            0
        );

        // Teardown
        teardown_test(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn dir_object() -> Result<(), UtilityError> {
        let test_name = "dir_object";
        // Start er up!
        let (metadata, content, forest, dir) = &mut setup_memory_test(test_name).await?;

        let private_ref = &store_dir(metadata, content, forest, dir).await?;
        let forest_cid = store_forest(forest, metadata, metadata).await?;
        let new_forest = &load_forest(&forest_cid, metadata).await?;
        let mut new_dir = load_dir(metadata, private_ref, new_forest).await?;
        // Assert equality
        assert_eq!(dir, &mut new_dir);
        // Teardown
        teardown_test(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn dir_content() -> Result<(), UtilityError> {
        let test_name = "dir_content";
        // Start er up!
        let (metadata, content, original_forest, original_dir) =
            &mut setup_memory_test(test_name).await?;

        // Grab the original file
        let original_file = original_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                original_forest,
                metadata,
                &mut thread_rng(),
            )
            .await
            .map_err(Box::from)?;

        // Get the content
        let original_content = original_file
            .get_content(original_forest, content)
            .await
            .map_err(Box::from)?;
        let private_ref = store_dir(metadata, content, original_forest, original_dir).await?;
        let forest_cid = store_forest(original_forest, metadata, metadata).await?;

        let mut new_forest = load_forest(&forest_cid, metadata).await?;
        let mut new_dir = load_dir(metadata, &private_ref, &new_forest).await?;
        // Assert equality
        assert_eq!(original_dir, &mut new_dir);

        let file = new_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                &mut new_forest,
                metadata,
                &mut thread_rng(),
            )
            .await
            .map_err(Box::from)?;
        // Get the content
        let new_content = file
            .get_content(original_forest, content)
            .await
            .map_err(Box::from)?;

        assert_eq!(original_content, new_content);

        // Teardown
        teardown_test(test_name).await
    }
}
