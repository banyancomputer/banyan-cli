use async_trait::async_trait;
use anyhow::Result;
use std::{borrow::Cow, io::Cursor};
use crate::blockstore::{TombBlockStore, BlockStore};
use crate::car::v2::CarV2;
use wnfs::libipld::{Cid, IpldCodec};


#[derive(Debug)]
/// CarV2 formatted memory blockstore
pub struct CarV2MemoryBlockStore {
    data: Vec<u8>,
    car: CarV2,
}

impl CarV2MemoryBlockStore {
    /// Create a new CarV2BlockStore from a readable stream
    pub fn new(vec: Vec<u8>) -> Result<Self> {
        // Read data
        let data = vec;
        // Load car
        let car = CarV2::read_bytes(Cursor::new(&data))?;
        Ok(Self { data, car })
    }
}

#[async_trait(?Send)]
/// WnfsBlockStore implementation for CarV2BlockStore
impl BlockStore for CarV2MemoryBlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, anyhow::Error> {
        let mut reader = Cursor::new(&self.data);
        let block = self.car.get_block(cid, &mut reader)?;
        Ok(Cow::Owned(block.content))
    }

    async fn put_block(&self, _: Vec<u8>, _: IpldCodec) -> Result<Cid, anyhow::Error> {
        panic!("not implemented")
    }
}

#[async_trait(?Send)]
/// TombBlockStore implementation for CarV2BlockStore -- needed in order to interact with the Fs
impl TombBlockStore for CarV2MemoryBlockStore {
    fn get_root(&self) -> Option<Cid> {
        self.car.get_root()
    }

    fn set_root(&self, _: &Cid) {
        panic!("not implemented")
    }

    async fn update_block(
        &self,
        _: &Cid,
        _: Vec<u8>,
        _: IpldCodec,
    ) -> Result<Cid, anyhow::Error> {
        panic!("not implemented")
    }
}