/// CAR based blockstore
pub mod carblockstore;
/// Disk based blockstore
pub mod diskblockstore;
/// Network based blockstore
pub mod networkblockstore;

/*
#[cfg(test)]
mod tests {
    use super::{
        carblockstore::CarBlockStore, diskblockstore::DiskBlockStore,
        networkblockstore::NetworkBlockStore,
    };
    use anyhow::Result;
    use std::{net::Ipv4Addr, path::PathBuf};
    use wnfs::common::blockstore::{bs_duplication_test, bs_retrieval_test, bs_serialization_test};

    #[tokio::test]
    async fn disk_blockstore() -> Result<()> {
        let dir = &PathBuf::from("test");
        // ensure_path_exists_and_is_dir(dir)?;
        let store = &DiskBlockStore::new(dir);
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    async fn car_blockstore() -> Result<()> {
        let dir = &PathBuf::from("test");
        // ensure_path_exists_and_is_dir(dir)?;
        let store = &CarBlockStore::new(dir, None);
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }

    #[tokio::test]
    async fn network_blockstore() -> Result<()> {
        let dir = &PathBuf::from("test");
        ensure_path_exists_and_is_dir(dir)?;
        let store = &NetworkBlockStore::new(Ipv4Addr::new(127, 0, 0, 1), 5001);
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }
}
*/
