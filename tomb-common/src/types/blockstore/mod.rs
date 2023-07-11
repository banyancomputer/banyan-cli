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
    use crate::types::blockstore::rootedmemoryblockstore::RootedMemoryBlockStore;

    use super::diskblockstore::DiskBlockStore;
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
        let store = &RootedMemoryBlockStore::new();
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }
}
