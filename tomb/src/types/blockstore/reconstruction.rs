use anyhow::Result;
use async_trait::async_trait;
use std::borrow::Cow;
use tomb_common::{banyan_api::blockstore::BanyanApiBlockStore, blockstore::RootedBlockStore};
use wnfs::{
    common::BlockStore,
    libipld::{Cid, IpldCodec},
};

use super::CarV2DiskBlockStore;

/// BlockStore which can intercept block retrievals from the Api and store them locally
#[derive(Debug)]
pub struct ReconstructionBlockStore {
    /// BlockStore for querying
    pub api: BanyanApiBlockStore,
    /// BlockStore for reconstructing
    pub reconstruction: CarV2DiskBlockStore,
}

#[async_trait(?Send)]
impl BlockStore for ReconstructionBlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        let block = self.api.get_block(cid).await?;
        self.reconstruction
            .put_block(
                block.clone().to_vec(),
                cid.codec().try_into().unwrap_or(IpldCodec::Raw),
            )
            .await?;
        Ok(block)
    }

    async fn put_block(&self, _: Vec<u8>, _: IpldCodec) -> Result<Cid> {
        todo!("not yet implemented")
    }
}

#[async_trait(?Send)]
impl RootedBlockStore for ReconstructionBlockStore {
    fn get_root(&self) -> Option<Cid> {
        self.reconstruction.get_root()
    }

    fn set_root(&self, root: &Cid) {
        self.reconstruction.set_root(root)
    }
}
