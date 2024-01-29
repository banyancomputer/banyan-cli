use crate::blockstore::RootedBlockStore;
use async_trait::async_trait;
use futures::executor::block_on;
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use std::borrow::Cow;
use tokio::sync::RwLock;
use wnfs::{
    common::MemoryBlockStore as WnfsMemoryBlockStore,
    libipld::{Cid, IpldCodec},
};

use super::{BanyanBlockStore, BlockStoreError};

#[derive(Debug, Default)]
/// Memory implementation of a RootedBlockStore
pub struct MemoryBlockStore {
    root: RwLock<Option<Cid>>,
    store: RwLock<WnfsMemoryBlockStore>,
}

unsafe impl Send for MemoryBlockStore {}
unsafe impl Sync for MemoryBlockStore {}

impl Clone for MemoryBlockStore {
    fn clone(&self) -> Self {
        MemoryBlockStore {
            root: RwLock::new(*block_on(self.root.read())),
            store: RwLock::new(block_on(self.store.read()).clone()),
        }
    }
}

impl Serialize for MemoryBlockStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let root = block_on(self.root.read());
        let store = block_on(self.store.read());
        let mut state = serializer.serialize_struct("MemoryBlockStore", 2)?;
        state.serialize_field("root", &*root)?;
        state.serialize_field("store", &*store)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for MemoryBlockStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct MemoryBlockStoreData {
            root: Option<Cid>,
            store: WnfsMemoryBlockStore,
        }

        let MemoryBlockStoreData { root, store } = MemoryBlockStoreData::deserialize(deserializer)?;
        Ok(MemoryBlockStore {
            root: RwLock::new(root),
            store: RwLock::new(store),
        })
    }
}

impl MemoryBlockStore {
    /// Creates a new in-memory block store.
    pub fn new() -> Self {
        Self::default()
    }
}

use wnfs::common::BlockStore;

#[async_trait]
impl BanyanBlockStore for MemoryBlockStore {
    /// Retrieves an array of bytes from the block store with given CID.
    /// TODO: rewrapped the COW here which was a bad decision. but... otherwise the rwlock gets mad... come back later
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, BlockStoreError> {
        let store = block_on(self.store.read());
        let block = block_on(store.get_block(cid)).map(Cow::into_owned);
        let b = block.map_err(|err| BlockStoreError::wnfs(Box::from(err)).clone())?;
        Ok(Cow::Owned(b))
    }

    /// Stores an array of bytes in the block store.
    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid, BlockStoreError> {
        let store = block_on(self.store.write());
        let block = block_on(store.put_block(bytes, codec));
        block.map_err(|err| BlockStoreError::wnfs(Box::from(err)))
    }
}

#[async_trait]
impl RootedBlockStore for MemoryBlockStore {
    async fn get_root(&self) -> Option<Cid> {
        *self.root.read().await
    }

    async fn set_root(&self, root: &Cid) {
        *self.root.write().await = Some(*root)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {
    use crate::blockstore::{
        BanyanBlockStore, BlockStoreError, MemoryBlockStore, RootedBlockStore,
    };
    use wnfs::{
        //common::blockstore::{bs_duplication_test, bs_retrieval_test, bs_serialization_test},
        common::{bs_duplication_test, bs_retrieval_test, bs_serialization_test},
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
        store.set_root(&cid).await;
        // Get the root
        let root = store.get_root().await;
        // Assert that the root is the same as the cid
        assert_eq!(root, Some(cid));
        Ok(())
    }
}
