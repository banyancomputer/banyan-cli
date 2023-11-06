mod api;
mod carv2_memory;
mod memory;
mod split;

/// Use the WnfsBlockStore and BlockStore traits to define a BlockStore
/// Makes it so that downstream crates don't need to know about the underlying traits
pub use crate::banyan_common::traits::blockstore::RootedBlockStore;
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
