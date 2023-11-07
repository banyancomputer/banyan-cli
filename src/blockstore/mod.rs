mod api;
#[cfg(not(target_arch = "wasm32"))]
mod carv2_disk;
mod carv2_memory;
mod memory;
#[cfg(not(target_arch = "wasm32"))]
mod multi_carv2_disk;
mod split;
#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub mod test;

/// Api BlockStore
pub use api::BanyanApiBlockStore;
#[cfg(not(target_arch = "wasm32"))]
pub use carv2_disk::CarV2DiskBlockStore;
/// Memory based CarV2 formatted BlockStore implementation
pub use carv2_memory::CarV2MemoryBlockStore;
/// Disk based BlockStore implementation
/// Memory based BlockStore implementation
pub use memory::MemoryBlockStore;
#[cfg(not(target_arch = "wasm32"))]
pub use multi_carv2_disk::MultiCarV2DiskBlockStore;
/// Split BlockStore
pub use split::DoubleSplitStore;
/// Use the WnfsBlockStore and BlockStore traits to define a BlockStore
/// Makes it so that downstream crates don't need to know about the underlying traits
pub use wnfs::common::blockstore::BlockStore;

use async_trait::async_trait;
use wnfs::libipld::Cid;
/// Wrap a BlockStore with additional functionality to get / set a root CID
#[async_trait(?Send)]
pub trait RootedBlockStore: BlockStore {
    /// Get the root CID
    fn get_root(&self) -> Option<Cid>;
    /// Set the root CID
    fn set_root(&self, root: &Cid);
}
