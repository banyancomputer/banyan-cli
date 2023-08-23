use crate::blockstore::{BlockStore, TombBlockStore};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cell::RefCell};
use wnfs::{
    common::MemoryBlockStore as WnfsMemoryBlockStore,
    libipld::{Cid, IpldCodec},
};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
/// Memory implementation of a TombBlockStore
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
impl BlockStore for MemoryBlockStore {
    /// Retrieves an array of bytes from the block store with given CID.
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        self.store.get_block(cid).await
    }

    /// Stores an array of bytes in the block store.
    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        self.store.put_block(bytes, codec).await
    }
}

#[async_trait(?Send)]
impl TombBlockStore for MemoryBlockStore {
    fn get_root(&self) -> Option<Cid> {
        *self.root.borrow()
    }

    fn set_root(&self, root: &Cid) {
        *self.root.borrow_mut() = Some(*root)
    }

    // There is no way to update content in a memory store
    async fn update_block(&self, cid: &Cid, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        // Grab bytes
        let existing_bytes = self.store.get_block(cid).await?.to_vec();
        // Assert length equality
        assert_eq!(existing_bytes.len(), bytes.len());
        // Put the block
        self.store.put_block(bytes, codec).await
    }
}
