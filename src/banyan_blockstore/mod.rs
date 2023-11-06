mod api;
mod carv2_memory;
mod memory;
mod split;

/// Use the WnfsBlockStore and BlockStore traits to define a BlockStore
/// Makes it so that downstream crates don't need to know about the underlying traits
pub use wnfs::common::blockstore::BlockStore;

/// Api BlockStore
pub use api::BanyanApiBlockStore;
/// Memory based CarV2 formatted BlockStore implementation
pub use carv2_memory::CarV2MemoryBlockStore;
/// Disk based BlockStore implementation
/// Memory based BlockStore implementation
pub use memory::MemoryBlockStore;
/// Split BlockStore
pub use split::DoubleSplitStore;

#[cfg(not(target_arch = "wasm32"))]
mod io;

#[cfg(not(target_arch = "wasm32"))]
pub use io::*;

use async_trait::async_trait;
use wnfs::libipld::Cid;
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
