use super::{BanyanBlockStore, BlockStoreError, RootedBlockStore};
use crate::LibipldError;
use async_trait::async_trait;
use std::borrow::Cow;
use wnfs::libipld::{Cid, IpldCodec};

/// Blockstore built over two
#[derive(Debug)]
pub struct DoubleSplitStore<'a, M: BanyanBlockStore, D: BanyanBlockStore> {
    primary: &'a M,
    secondary: &'a D,
}

#[async_trait]
impl<M: RootedBlockStore, D: BanyanBlockStore> RootedBlockStore for DoubleSplitStore<'_, M, D> {
    async fn get_root(&self) -> Option<Cid> {
        self.primary.get_root().await
    }

    async fn set_root(&self, root: &Cid) {
        self.primary.set_root(root).await
    }
}

#[async_trait]
impl<M: BanyanBlockStore, D: BanyanBlockStore> BanyanBlockStore for DoubleSplitStore<'_, M, D> {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, BlockStoreError> {
        match BanyanBlockStore::get_block(self.primary, cid).await {
            Ok(blk) => Ok(blk),
            Err(_) => BanyanBlockStore::get_block(self.secondary, cid)
                .await
                .map_err(|err| BlockStoreError::wnfs(Box::from(err))),
        }
    }

    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid, BlockStoreError> {
        // TODO: this needs to be .ok() since some workflows use a BanyanApiBlockStore as the secondary
        // and it does not implement put_block ...
        BanyanBlockStore::put_block(self.secondary, bytes.clone(), codec)
            .await
            .ok();
        BanyanBlockStore::put_block(self.primary, bytes, codec)
            .await
            .map_err(|err| BlockStoreError::wnfs(Box::from(err)))
    }
}

impl<'a, M: BanyanBlockStore, D: BanyanBlockStore> DoubleSplitStore<'a, M, D> {
    /// Create a new split BlockStore
    pub fn new(primary: &'a M, secondary: &'a D) -> Self {
        Self { primary, secondary }
    }
}

#[async_trait(?Send)]
impl<M: BanyanBlockStore, D: BanyanBlockStore> wnfs::common::BlockStore
    for DoubleSplitStore<'_, M, D>
{
    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid, LibipldError> {
        BanyanBlockStore::put_block(self, bytes, codec)
            .await
            .map_err(|err| LibipldError::msg(err.to_string()))
    }

    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, LibipldError> {
        BanyanBlockStore::get_block(self, cid)
            .await
            .map_err(|err| LibipldError::msg(err.to_string()))
    }
}
