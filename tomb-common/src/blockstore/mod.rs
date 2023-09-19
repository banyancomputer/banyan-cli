/// Use the WnfsBlockStore and BlockStore traits to define a BlockStore
/// Makes it so that downstream crates don't need to know about the underlying traits
pub use crate::traits::blockstore::RootedBlockStore;
pub use wnfs::common::blockstore::BlockStore;

#[cfg(not(target_arch = "wasm32"))]
/// Disk based CarV2 formatted BlockStore implementation
pub mod carv2_disk;
/// Memory based CarV2 formatted BlockStore implementation
pub mod carv2_memory;
// pub mod carv2_staging;
/// Disk based BlockStore implementation
pub mod disk;
/// Memory based BlockStore implementation
pub mod memory;
#[cfg(not(target_arch = "wasm32"))]
/// Multi-file CarV2 formatted BlockStore implementation
pub mod multi_carv2_disk;
