/// CAR based blockstore
pub mod car;
pub mod carblockstore;
/// Disk based blockstore
pub mod diskblockstore;
/// Network based blockstore
pub mod networkblockstore;

#[cfg(test)]
mod tests {
    use crate::types::blockstore::car::carv1blockstore::CarV1BlockStore;

    use super::{
        carblockstore::CarBlockStore, diskblockstore::DiskBlockStore,
        networkblockstore::NetworkBlockStore,
    };
    use anyhow::Result;
    use std::{
        fs::{create_dir_all, File},
        path::PathBuf,
    };
    use wnfs::common::blockstore::{bs_duplication_test, bs_retrieval_test, bs_serialization_test};

    #[tokio::test]
    async fn disk_blockstore() -> Result<()> {
        let dir = &PathBuf::from("test");
        create_dir_all(dir)?;
        let store = &DiskBlockStore::new(dir);
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    async fn car_blockstore() -> Result<()> {
        let dir = &PathBuf::from("test");
        create_dir_all(dir)?;
        let store = &CarBlockStore::new(dir, None);
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    async fn carv1_blockstore() -> Result<()> {
        let dir = &PathBuf::from("test");
        create_dir_all(dir)?;
        let car_path = dir.join("example.car");
        File::create(&car_path)?;
        let store = &CarV1BlockStore::new(&car_path)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    async fn network_blockstore() -> Result<()> {
        let dir = &PathBuf::from("test");
        create_dir_all(dir)?;
        let store = &NetworkBlockStore::new("http://127.0.0.1", 5001);
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }
}
