use anyhow::Result;
use async_trait::async_trait;
use wnfs::{
    common::blockstore::BlockStore as WnfsBlockStore,
    libipld::{Cid, IpldCodec},
};

// TODO: Use better error types
/// Wrap a WnfsBlockStore with additional functionality
#[async_trait(?Send)]
pub trait TombBlockStore: WnfsBlockStore {
    /// Get the root CID
    fn get_root(&self) -> Option<Cid>;
    /// Set the root CID
    fn set_root(&self, root: &Cid);
    /// Update the bytes of a block in-place
    async fn update_block(&self, cid: &Cid, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid>;
}