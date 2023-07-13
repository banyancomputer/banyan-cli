/// CAR readers and writers
pub mod car;
/// Disk based BlockStore
pub mod diskblockstore;
/// Network based BlockStore
pub mod networkblockstore;
/// Tomb BlockStore trait
pub mod tombblockstore;
/// Memory implementation of Tomb BlockStore trait
pub mod tombmemoryblockstore;

#[cfg(test)]
mod test {
    use super::diskblockstore::DiskBlockStore;
    use crate::types::blockstore::tombmemoryblockstore::TombMemoryBlockStore;
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
    async fn rootedmemoryblockstore() -> Result<()> {
        let store = &TombMemoryBlockStore::new();
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }
}
