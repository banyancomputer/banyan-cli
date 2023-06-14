/// CAR based blockstore
pub mod car;
/// Disk based blockstore
pub mod diskblockstore;
/// Network based blockstore
pub mod networkblockstore;

#[cfg(test)]
mod tests {
    use crate::types::blockstore::car::{
        carv1blockstore::CarV1BlockStore, carv2blockstore::CarV2BlockStore,
    };

    use super::{diskblockstore::DiskBlockStore, networkblockstore::NetworkBlockStore};
    use anyhow::Result;
    use std::{
        fs::{create_dir_all, File},
        path::{PathBuf, Path},
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
    async fn carv1blockstore() -> Result<()> {
        let dir = Path::new("test");
        create_dir_all(dir)?;
        let car_path = dir.join("CARv1BlockStore.car");
        let store = &CarV1BlockStore::new(&car_path, None)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        // bs_serialization_test(store).await
        Ok(())
    }

    #[tokio::test]
    async fn carv2blockstore() -> Result<()> {
        let dir = &PathBuf::from("test");
        create_dir_all(dir)?;
        let car_path = dir.join("CARv2.car");
        File::create(&car_path)?;
        let store = &CarV2BlockStore::new(&car_path)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    async fn networkblockstore() -> Result<()> {
        let dir = &PathBuf::from("test");
        create_dir_all(dir)?;
        let store = &NetworkBlockStore::new("http://127.0.0.1", 5001);
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }
}
