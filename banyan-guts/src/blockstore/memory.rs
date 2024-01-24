use crate::blockstore::RootedBlockStore;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cell::RefCell};
use wnfs::{
    common::{BlockStore, MemoryBlockStore as WnfsMemoryBlockStore},
    libipld::{Cid, IpldCodec},
};

use super::{BanyanBlockStore, BlockStoreError};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
/// Memory implementation of a RootedBlockStore
pub struct MemoryBlockStore {
    root: RefCell<Option<Cid>>,
    store: WnfsMemoryBlockStore,
}

impl MemoryBlockStore {
    /// Creates a new in-memory block store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait(?Send)]
impl BanyanBlockStore for MemoryBlockStore {
    /// Retrieves an array of bytes from the block store with given CID.
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, BlockStoreError> {
        self.store
            .get_block(cid)
            .await
            .map_err(|err| BlockStoreError::wnfs(Box::from(err)))
    }

    /// Stores an array of bytes in the block store.
    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid, BlockStoreError> {
        self.store
            .put_block(bytes, codec)
            .await
            .map_err(|err| BlockStoreError::wnfs(Box::from(err)))
    }
}

#[async_trait(?Send)]
impl RootedBlockStore for MemoryBlockStore {
    fn get_root(&self) -> Option<Cid> {
        *self.root.borrow()
    }

    fn set_root(&self, root: &Cid) {
        *self.root.borrow_mut() = Some(*root)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
    use crate::blockstore::{
        BanyanBlockStore, BlockStoreError, MemoryBlockStore, RootedBlockStore,
    };
    use wnfs::{
        common::blockstore::{bs_duplication_test, bs_retrieval_test, bs_serialization_test},
        libipld::IpldCodec,
    };

    #[tokio::test]
    async fn memory_blockstore() -> Result<(), BlockStoreError> {
        let store = &MemoryBlockStore::default();
        bs_retrieval_test(store).await.map_err(Box::from)?;
        bs_duplication_test(store).await.map_err(Box::from)?;
        bs_serialization_test(store).await.map_err(Box::from)?;
        Ok(())
    }

    #[tokio::test]
    async fn memory_rooted_blockstore() -> Result<(), BlockStoreError> {
        let store = &MemoryBlockStore::default();
        // Put a block in the store
        let cid = store
            .put_block(vec![1, 2, 3], IpldCodec::Raw)
            .await
            .map_err(Box::from)?;
        // Set the root
        store.set_root(&cid);
        // Get the root
        let root = store.get_root();
        // Assert that the root is the same as the cid
        assert_eq!(root, Some(cid));
        Ok(())
    }
}
