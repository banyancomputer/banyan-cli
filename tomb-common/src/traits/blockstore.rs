use async_trait::async_trait;
use wnfs::common::blockstore::BlockStore;
use libipld::Cid;

// TODO: Use better error types
/// Wrap a BlockStore with additional functionality to get / set a root CID
#[async_trait(?Send)]
pub trait RootedBlockStore: BlockStore {
    /// Get the root CID
    fn get_root(&self) -> Option<Cid>;
    /// Set the root CID
    fn set_root(&self, root: &Cid);
    // TODO: This is never called, and is not a consistent applicable to all blockstores
    // Commenting out for now but eventually we should remove it
    // / Update the bytes of a block in-place
    // async fn update_block(&self, cid: &Cid, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid>;
}
