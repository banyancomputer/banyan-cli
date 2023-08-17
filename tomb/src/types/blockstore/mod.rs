/// CARv1
pub mod carv1;
/// CARv2
pub mod carv2;
/// CAR Disk BlockStore errors
pub mod error;
/// Multifile CARv2 BLockStore
pub mod multi;

#[cfg(test)]
mod test {
    use crate::types::blockstore::{carv1, carv2, multi};
    use anyhow::Result;
    use serial_test::serial;
    use std::{fs::remove_dir_all, path::Path};
    use tomb_common::utils::tests::car_test_setup;
    use wnfs::common::blockstore::{bs_duplication_test, bs_retrieval_test, bs_serialization_test};

    #[tokio::test]
    #[serial]
    async fn carv1blockstore() -> Result<()> {
        let car_path = &car_test_setup(1, "basic", "blockstore")?;
        let store = &carv1::BlockStore::new(car_path)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    #[serial]
    async fn carv2blockstore() -> Result<()> {
        let car_path = &car_test_setup(2, "indexless", "blockstore")?;
        let store = &carv2::BlockStore::new(car_path)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    #[serial]
    async fn multifileblockstore() -> Result<()> {
        let test_dir = &Path::new("test").join("car").join("multifile_blockstore");
        if test_dir.exists() {
            remove_dir_all(test_dir)?;
        }
        let mut store = multi::BlockStore::new(test_dir)?;
        store.add_delta()?;
        bs_retrieval_test(&store).await?;
        bs_duplication_test(&store).await?;
        bs_serialization_test(&store).await
    }
}
