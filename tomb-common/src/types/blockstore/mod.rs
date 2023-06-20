/// CAR based blockstore
pub mod car;
/// Disk based blockstore
pub mod diskblockstore;
/// Network based blockstore
pub mod networkblockstore;

#[cfg(test)]
mod tests {
    use super::{diskblockstore::DiskBlockStore, networkblockstore::NetworkBlockStore};
    use crate::types::blockstore::car::{
        carv1::carv1blockstore::CarV1BlockStore, carv2::carv2blockstore::CarV2BlockStore,
    };
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
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv1-basic.car");
        let new_path = Path::new("test").join("carv1-basic-v1.car");
        std::fs::copy(existing_path, &new_path)?;
        let store = &CarV1BlockStore::new(&new_path, None)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    #[serial]
    async fn carv2blockstore() -> Result<()> {
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv2-basic.car");
        let new_path = Path::new("test").join("carv2-basic-v2.car");
        std::fs::copy(existing_path, &new_path)?;
        let store = &CarV2BlockStore::new(&new_path)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        // bs_serialization_test(store).await
        Ok(())
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
