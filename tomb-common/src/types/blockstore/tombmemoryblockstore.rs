use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cell::RefCell};
use wnfs::{
    common::{BlockStore as WnfsBlockStore, MemoryBlockStore, BlockStoreError},
    libipld::{Cid, IpldCodec},
};

use super::tombblockstore::TombBlockStore;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RootedMemoryBlockStore {
    root: RefCell<Option<Cid>>,
    store: MemoryBlockStore,
}

impl RootedMemoryBlockStore {
    /// Creates a new in-memory block store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait(?Send)]
impl WnfsBlockStore for RootedMemoryBlockStore {
    /// Retrieves an array of bytes from the block store with given CID.
    async fn get_block(&self, cid: &Cid) -> Result<Cow<Vec<u8>>> {
        self.store.get_block(cid).await
    }

    /// Stores an array of bytes in the block store.
    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        self.store.put_block(bytes, codec).await
    }
}

#[async_trait(?Send)]
impl TombBlockStore for RootedMemoryBlockStore {
    fn get_root(&self) -> Option<Cid> {
        *self.root.borrow()
    }

    fn set_root(&self, root: &Cid) {
        *self.root.borrow_mut() = Some(*root)
    }

    // There is no way to update content in a memory store
    async fn update_content(&self, _: &Cid, _: Vec<u8>, _: IpldCodec) -> Result<Cid> {
        Err(BlockStoreError::LockPoisoned.into())
    }
}
