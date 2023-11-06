mod carv2_memory;
mod memory;
mod split;

/// Use the WnfsBlockStore and BlockStore traits to define a BlockStore
/// Makes it so that downstream crates don't need to know about the underlying traits
pub use crate::banyan_common::traits::blockstore::RootedBlockStore;
pub use wnfs::common::blockstore::BlockStore;

/// Disk based BlockStore implementation
/// Memory based BlockStore implementation
pub use memory::MemoryBlockStore;
/// Memory based CarV2 formatted BlockStore implementation
pub use carv2_memory::CarV2MemoryBlockStore;
/// Split blockstore
pub use split::DoubleSplitStore;

// #[cfg(not(target_arch="wasm32"))]
pub mod disk;
pub mod carv2_disk;
pub mod multi_carv2_disk;

pub use disk::DiskBlockStore;
pub use carv2_disk::CarV2DiskBlockStore;
pub use multi_carv2_disk::MultiCarV2DiskBlockStore;
