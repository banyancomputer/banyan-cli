use std::borrow::Cow;
use wnfs::{
    common::BlockStore,
    libipld::{Cid, IpldCodec},
};

use super::RootedBlockStore;

/// Blockstore built over two
#[derive(Debug)]
pub struct DoubleSplitStore<'a, M: BlockStore, D: BlockStore> {
    primary: &'a M,
    secondary: &'a D,
}

impl<M: RootedBlockStore, D: BlockStore> RootedBlockStore for DoubleSplitStore<'_, M, D> {
    fn get_root(&self) -> Option<Cid> {
        self.primary.get_root()
    }

    fn set_root(&self, root: &Cid) {
        self.primary.set_root(root)
    }
}

#[async_trait::async_trait(?Send)]
impl<M: BlockStore, D: BlockStore> BlockStore for DoubleSplitStore<'_, M, D> {
    async fn get_block(&self, cid: &Cid) -> anyhow::Result<Cow<'_, Vec<u8>>> {
        match self.primary.get_block(cid).await {
            Ok(blk) => Ok(blk),
            Err(_) => self.secondary.get_block(cid).await,
        }
    }

    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> anyhow::Result<Cid> {
        self.secondary.put_block(bytes.clone(), codec).await.ok();
        self.primary.put_block(bytes, codec).await
    }
}

impl<'a, M: BlockStore, D: BlockStore> DoubleSplitStore<'a, M, D> {
    /// Create a new split BlockStore
    pub fn new(primary: &'a M, secondary: &'a D) -> Self {
        Self { primary, secondary }
    }
}
