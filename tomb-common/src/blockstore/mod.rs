/// Use the WnfsBlockStore and BlockStore traits to define a BlockStore
/// Makes it so that downstream crates don't need to know about the underlying traits
pub use crate::traits::blockstore::TombBlockStore;
pub use wnfs::common::blockstore::BlockStore;

/// Memory based CarV2 formatted BlockStore implementation
pub mod carv2_memory;
/// Disk based BlockStore implementation
pub mod disk;
/// Memory based BlockStore implementation
pub mod memory;
/// Network based BlockStore implementation
pub mod network;

#[cfg(test)]
mod test {
    use super::disk::DiskBlockStore;
    use crate::blockstore::memory::MemoryBlockStore;
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
        let store = &MemoryBlockStore::new();
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }
}
