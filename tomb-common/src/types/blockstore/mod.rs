/// CAR based blockstore
pub mod car;
/// Disk based blockstore
pub mod diskblockstore;
/// Network based blockstore
pub mod networkblockstore;

#[cfg(test)]
mod tests {
    use super::{diskblockstore::DiskBlockStore, networkblockstore::NetworkBlockStore};
    use crate::{types::blockstore::car::{
        carv1::carv1blockstore::CarV1BlockStore, carv2::carv2blockstore::CarV2BlockStore,
    }, utils::tests::car_setup};
    use anyhow::Result;
    use serial_test::serial;
    use std::{
        fs::create_dir_all,
        path::{Path, PathBuf},
    };
    use wnfs::common::blockstore::{bs_duplication_test, bs_retrieval_test, bs_serialization_test};

    #[tokio::test]
    async fn diskblockstore() -> Result<()> {
        let dir = &PathBuf::from("test");
        create_dir_all(dir)?;
        let store = &DiskBlockStore::new(dir);
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    #[serial]
    async fn carv1blockstore() -> Result<()> {
        let car_path = &car_setup(1, "basic", "blockstore")?;
        let store = &CarV1BlockStore::new(car_path)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    #[serial]
    async fn carv2blockstore() -> Result<()> {
        let car_path = &car_setup(2, "indexless", "blockstore")?;
        let store = &CarV2BlockStore::new(car_path)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    async fn networkblockstore() -> Result<()> {
        let dir = &PathBuf::from("test");
        create_dir_all(dir)?;
        let store = &NetworkBlockStore::new("http://127.0.0.1:5001");
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }
}
