pub mod carv1;
pub mod carv2;

#[cfg(test)]
mod test {
    use std::path::Path;

    use crate::types::blockstore::{
        carv1,
        carv2::{self, multifile::MultifileBlockStore},
    };
    use anyhow::Result;
    use serial_test::serial;
    use tomb_common::utils::test::car_setup;
    use wnfs::common::blockstore::{bs_duplication_test, bs_retrieval_test, bs_serialization_test};

    #[tokio::test]
    #[serial]
    async fn carv1blockstore() -> Result<()> {
        let car_path = &car_setup(1, "basic", "blockstore")?;
        let store = &carv1::blockstore::BlockStore::new(car_path)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    #[serial]
    async fn carv2blockstore() -> Result<()> {
        let car_path = &car_setup(2, "indexless", "blockstore")?;
        let store = &carv2::blockstore::BlockStore::new(car_path)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    #[serial]
    async fn multifileblockstore() -> Result<()> {
        let test_dir = &Path::new("test").join("car").join("multifile_blockstore");
        let store = &MultifileBlockStore::new(test_dir)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await
        // Serialization needs to be tested separately for multifile BlockStores,
        // Because we expect that they actually change between serialization and deserialization.
    }
}
