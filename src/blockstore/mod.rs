mod api;
#[cfg(not(target_arch = "wasm32"))]
mod carv2_disk;
mod carv2_memory;
mod error;
mod memory;
#[cfg(not(target_arch = "wasm32"))]
mod multi_carv2_disk;
mod split;
mod wnfs_trait;

pub use api::BanyanApiBlockStore;
#[cfg(not(target_arch = "wasm32"))]
pub use carv2_disk::CarV2DiskBlockStore;
pub use carv2_memory::CarV2MemoryBlockStore;
pub(crate) use error::BlockStoreError;
pub use memory::MemoryBlockStore;
#[cfg(not(target_arch = "wasm32"))]
pub use multi_carv2_disk::MultiCarV2DiskBlockStore;
pub use split::DoubleSplitStore;
/// Makes it so that downstream crates don't need to know about the underlying trait
pub use wnfs_trait::BanyanBlockStore;

use async_trait::async_trait;
use wnfs::libipld::Cid;
/// Wrap a BlockStore with additional functionality to get / set a root CID
#[async_trait(?Send)]
pub trait RootedBlockStore: BanyanBlockStore {
    /// Get the root CID
    fn get_root(&self) -> Option<Cid>;
    /// Set the root CID
    fn set_root(&self, root: &Cid);
}
