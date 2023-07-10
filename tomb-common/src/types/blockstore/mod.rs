/// CAR based blockstore
pub mod car;
/// Disk based blockstore
pub mod diskblockstore;
/// Network based blockstore
pub mod networkblockstore;
/// Trait
pub mod rootedblockstore;
pub mod rootedmemoryblockstore;

#[cfg(test)]
mod test {
    use super::{diskblockstore::DiskBlockStore, networkblockstore::NetworkBlockStore};
    use anyhow::Result;
    use std::{fs::create_dir_all, path::PathBuf};
    use wnfs::common::blockstore::{bs_duplication_test, bs_retrieval_test, bs_serialization_test};

    #[tokio::test]
    async fn diskblockstore() -> Result<()> {
        let dir = &PathBuf::from("test").join("diskblockstore");
        create_dir_all(dir)?;
        let store = &DiskBlockStore::new(dir);
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    #[ignore]
    async fn networkblockstore() -> Result<()> {
        let dir = &PathBuf::from("test");
        create_dir_all(dir)?;
        let store = &NetworkBlockStore::new("http://127.0.0.1:5001")?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }
}
