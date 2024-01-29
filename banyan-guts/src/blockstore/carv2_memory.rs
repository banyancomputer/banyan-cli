use crate::{
    blockstore::{BlockStoreError, RootedBlockStore},
    car::{error::CarError, v1::Block, v2::CarV2},
};
use async_trait::async_trait;
use futures::executor::block_on;
use serde::de::Error;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::{borrow::Cow, io::Cursor};
use tokio::sync::RwLock;
use wnfs::libipld::{Cid, IpldCodec};

use super::BanyanBlockStore;

#[derive(Debug)]
/// CarV2 formatted memory blockstore
pub struct CarV2MemoryBlockStore {
    data: RwLock<Cursor<Vec<u8>>>,
    pub(crate) car: CarV2,
}

impl PartialEq for CarV2MemoryBlockStore {
    fn eq(&self, other: &Self) -> bool {
        let self_data = futures::executor::block_on(self.data.read()).clone();
        let other_data = futures::executor::block_on(other.data.read()).clone();
        self_data == other_data && self.car == other.car
    }
}

impl TryFrom<Vec<u8>> for CarV2MemoryBlockStore {
    type Error = CarError;

    fn try_from(vec: Vec<u8>) -> Result<Self, Self::Error> {
        let mut store = Self {
            data: RwLock::new(Cursor::new(vec![])),
            car: block_on(CarV2::new(Cursor::new(vec![])))?,
        };

        {
            let data: &mut Cursor<Vec<u8>> = &mut block_on(store.data.write());
            // Write all the vec to data
            data.write_all(&vec)?;
            // Load the car
            store.car = block_on(CarV2::read_bytes(data))?;
        }

        Ok(store)
    }
}

impl CarV2MemoryBlockStore {
    /// Create a new CarV2BlockStore from a readable stream
    pub async fn new() -> Result<Self, BlockStoreError> {
        // Read data
        let mut rw = Cursor::new(<Vec<u8>>::new());
        let car = CarV2::new(&mut rw).await?;
        // Wrap the vec in a RefCell and add it to self
        let data = RwLock::new(rw);
        Ok(Self { data, car })
    }

    /// Get the size of the data underlying the CarV1
    pub async fn data_size(&self) -> u64 {
        self.car.data_size().await
    }

    /// Manually save the data to the cursor in place
    pub async fn save(&self) -> Result<(), CarError> {
        let rw: &mut Cursor<Vec<u8>> = &mut *self.data.write().await;
        self.car.write_bytes(rw).await?;
        Ok(())
    }

    /// Get a reader to the data underlying the CarV2
    pub async fn get_data(&self) -> Vec<u8> {
        self.save().await.unwrap();
        self.data.read().await.clone().into_inner()
    }
}

#[async_trait]
/// WnfsBlockStore implementation for CarV2BlockStore
impl BanyanBlockStore for CarV2MemoryBlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, BlockStoreError> {
        let reader: &mut Cursor<Vec<u8>> = &mut *self.data.write().await;
        let block = self.car.get_block(cid, reader).await?;
        Ok(Cow::Owned(block.content))
    }

    async fn put_block(&self, content: Vec<u8>, codec: IpldCodec) -> Result<Cid, BlockStoreError> {
        let writer: &mut Cursor<Vec<u8>> = &mut *self.data.write().await;
        let block = Block::new(content, codec)?;
        self.car.put_block(&block, writer).await?;
        Ok(block.cid)
    }
}

#[async_trait]
/// RootedBlockStore implementation for CarV2BlockStore -- needed in order to interact with the Fs
impl RootedBlockStore for CarV2MemoryBlockStore {
    async fn get_root(&self) -> Option<Cid> {
        self.car.get_root().await
    }

    async fn set_root(&self, root: &Cid) {
        self.car.set_root(root).await
    }
}

impl Serialize for CarV2MemoryBlockStore {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        block_on(self.get_data()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CarV2MemoryBlockStore {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data = <Vec<u8>>::deserialize(deserializer)?;
        Self::try_from(data).map_err(D::Error::custom)
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
    use crate::blockstore::{BanyanBlockStore, BlockStoreError, RootedBlockStore};
    use wnfs::{
        common::{bs_duplication_test, bs_retrieval_test, bs_serialization_test},
        libipld::IpldCodec,
    };

    use super::CarV2MemoryBlockStore;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn put_block() -> Result<(), BlockStoreError> {
        let store = CarV2MemoryBlockStore::new().await?;
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store.put_block(kitty_bytes.clone(), IpldCodec::Raw).await?;
        let new_kitty_bytes = store.get_block(&kitty_cid).await?.to_vec();
        assert_eq!(kitty_bytes, new_kitty_bytes);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn from_scratch() -> Result<(), BlockStoreError> {
        // Open
        let original = CarV2MemoryBlockStore::new().await?;
        // Put a block in
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = original
            .put_block(kitty_bytes.clone(), IpldCodec::Raw)
            .await?;
        // Insert root
        original.set_root(&kitty_cid).await;
        // Save
        let all_data = original.get_data().await;
        // Reopen
        let reconstructed = CarV2MemoryBlockStore::try_from(all_data)?;
        // Ensure content is still there
        assert_eq!(
            kitty_cid,
            original.get_root().await.expect("no root in CAR")
        );
        assert_eq!(kitty_bytes, original.get_block(&kitty_cid).await?.to_vec());
        // Assert equality
        assert_eq!(original, reconstructed);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn carv2memoryblockstore() -> Result<(), BlockStoreError> {
        let store = &CarV2MemoryBlockStore::new().await?;
        bs_retrieval_test(store)
            .await
            .map_err(|err| BlockStoreError::wnfs(Box::from(err)))?;
        bs_duplication_test(store)
            .await
            .map_err(|err| BlockStoreError::wnfs(Box::from(err)))?;
        bs_serialization_test(store)
            .await
            .map_err(|err| BlockStoreError::wnfs(Box::from(err)))?;
        Ok(())
    }
}
