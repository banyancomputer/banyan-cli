use std::borrow::Cow;

use async_trait::async_trait;
use wnfs::libipld::{Cid, IpldCodec};

use crate::LibipldError;

use crate::blockstore::{
    BanyanApiBlockStore, BlockStoreError, CarV2MemoryBlockStore, MemoryBlockStore,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::blockstore::{CarV2DiskBlockStore, MultiCarV2DiskBlockStore};

#[async_trait]
pub trait BanyanBlockStore: Sync + Send {
    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid, BlockStoreError>;
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, BlockStoreError>;
}

macro_rules! impl_wnfs_blockstore {
    ($structname: ident) => {
        #[async_trait(?Send)]
        impl wnfs::common::BlockStore for $structname {
            async fn put_block(
                &self,
                bytes: Vec<u8>,
                codec: IpldCodec,
            ) -> Result<Cid, LibipldError> {
                BanyanBlockStore::put_block(self, bytes, codec)
                    .await
                    .map_err(|err| LibipldError::msg(err.to_string()))
            }

            async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, LibipldError> {
                BanyanBlockStore::get_block(self, cid)
                    .await
                    .map_err(|err| LibipldError::msg(err.to_string()))
            }
        }
    };
}

impl_wnfs_blockstore!(BanyanApiBlockStore);
impl_wnfs_blockstore!(MemoryBlockStore);
impl_wnfs_blockstore!(CarV2MemoryBlockStore);
#[cfg(not(target_arch = "wasm32"))]
impl_wnfs_blockstore!(CarV2DiskBlockStore);
#[cfg(not(target_arch = "wasm32"))]
impl_wnfs_blockstore!(MultiCarV2DiskBlockStore);
