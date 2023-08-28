/// Use the WnfsBlockStore and BlockStore traits to define a BlockStore
/// Makes it so that downstream crates don't need to know about the underlying traits
pub use crate::traits::blockstore::RootedBlockStore;
pub use wnfs::common::blockstore::BlockStore;

#[cfg(feature = "native")]
/// Disk based CarV2 formatted BlockStore implementation
pub mod carv2_disk;
/// Memory based CarV2 formatted BlockStore implementation
pub mod carv2_memory;
/// Disk based BlockStore implementation
pub mod disk;
/// Memory based BlockStore implementation
pub mod memory;
#[cfg(feature = "native")]
/// Multi-file CarV2 formatted BlockStore implementation
pub mod multi_carv2_disk;
/// Network based BlockStore implementation
pub mod network;
