use anyhow::Result;
use async_trait::async_trait;
use wnfs::{
    common::blockstore::BlockStore as WnfsBlockStore,
    libipld::{Cid, IpldCodec},
};

#[async_trait(?Send)]
/// Additional functionality that we expect out of our BlockStores
pub trait TombBlockStore: WnfsBlockStore {
    /// Get the root CID
    fn get_root(&self) -> Option<Cid>;
    /// Set the root CID
    fn set_root(&self, root: &Cid);
    /// Update the bytes of a block in-place
    async fn update_content(&self, cid: &Cid, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid>;
}
