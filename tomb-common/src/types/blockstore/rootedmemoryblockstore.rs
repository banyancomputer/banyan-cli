use anyhow::Result;
use std::{cell::RefCell, borrow::Cow};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use wnfs::{libipld::{Cid, IpldCodec}, common::{MemoryBlockStore, BlockStore as WnfsBlockStore}};

use super::rootedblockstore::RootedBlockStore;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RootedMemoryBlockStore {
    root: RefCell<Option<Cid>>,
    store: MemoryBlockStore
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

impl RootedBlockStore for RootedMemoryBlockStore {
    fn get_root(&self) -> Option<Cid> {
        *self.root.borrow()
    }

    fn set_root(&self, root: &Cid) {
        *self.root.borrow_mut() = Some(*root)
    }
}