use crate::{
    blockstore::{BlockStoreError, RootedBlockStore},
    car::{error::CarError, v1::Block, v2::CarV2},
    utils::{get_read, get_read_write, get_write},
};
use async_trait::async_trait;
use serde::{de::Error as DeError, Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs::File,
    path::{Path, PathBuf},
};
use wnfs::libipld::{Cid, IpldCodec};

use super::BanyanBlockStore;

/// CarV2DiskBlockStore implementation using File IO
#[derive(Debug, PartialEq, Clone)]
pub struct CarV2DiskBlockStore {
    /// CarV2 file path
    pub path: PathBuf,
    /// CarV2v2
    pub car: CarV2,
}

impl CarV2DiskBlockStore {
    /// Create a new CarV2DiskBlockStore at a given path
    pub async fn new(path: &Path) -> Result<Self, BlockStoreError> {
        if path.exists() {
            return Err(BlockStoreError::exists(path));
        }
        // Grab read and write
        let mut rw = get_read_write(path)?;
        // Create new
        let store = CarV2DiskBlockStore {
            path: path.to_path_buf(),
            car: CarV2::new(&mut rw).await?,
        };
        // Return Ok
        Ok(store)
    }

    /// LOad a new CARv2DiskBlockStore from a given path
    pub async fn load(path: &Path) -> Result<Self, BlockStoreError> {
        // If the path is a directory
        if path.is_dir() {
            return Err(BlockStoreError::missing_file(path));
        }

        // If the file is already a valid CARv2
        let mut file = File::open(path)?;
        let car = CarV2::read_bytes(&mut file).await?;
        Ok(Self {
            path: path.to_path_buf(),
            car,
        })
    }

    /// Save the CarV2 CarV2DiskBlockStore to disk
    pub async fn to_disk(&self) -> Result<(), CarError> {
        self.car.write_bytes(&mut get_read_write(&self.path)?).await
    }

    /// Get the size of the underlying CARv1
    pub async fn data_size(&self) -> u64 {
        self.car.data_size().await
    }
}

#[async_trait]
impl BanyanBlockStore for CarV2DiskBlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, BlockStoreError> {
        // Open the file in read-only mode
        let mut file = get_read(&self.path)?;
        // Perform the block read
        let block: Block = self.car.get_block(cid, &mut file).await?;
        // Return its contents
        Ok(Cow::Owned(block.content))
    }

    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid, BlockStoreError> {
        // Create a block with this content
        let block = Block::new(bytes, codec)?;
        // If this CID already exists in the store
        if self.get_block(&block.cid).await.is_ok() {
            // Return OK
            Ok(block.cid)
        }
        // If this needs to be appended to the CarV2v1
        else {
            // Open the file in append mode
            let mut file = get_write(&self.path)?;
            // Put the block
            self.car.put_block(&block, &mut file).await?;
            // Return Ok with block CID
            Ok(block.cid)
        }
    }
}

#[async_trait]
impl RootedBlockStore for CarV2DiskBlockStore {
    async fn set_root(&self, root: &Cid) {
        self.car.set_root(root).await;
        self.to_disk().await.expect("failed to write to disk");
    }

    async fn get_root(&self) -> Option<Cid> {
        self.car.get_root().await
    }
}

impl Serialize for CarV2DiskBlockStore {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize the Path
        self.path.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CarV2DiskBlockStore {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Grab the Path
        let path = PathBuf::deserialize(deserializer)?;
        // If we successfully load ourself from disk
        if let Ok(new_store) = futures::executor::block_on(Self::load(&path)) {
            // Return loaded object
            Ok(new_store)
        } else {
            // Create a new CarV2 Error
            Err(DeError::custom("Failed to load from disk"))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        blockstore::{BanyanBlockStore, BlockStoreError, CarV2DiskBlockStore, RootedBlockStore},
        utils::testing::blockstores::car_test_setup,
    };
    use serial_test::serial;
    use std::{fs::remove_file, path::Path, str::FromStr};
    //use wnfs::common::blockstore::{bs_duplication_test, bs_retrieval_test};
    use wnfs::{
        common::{bs_duplication_test, bs_retrieval_test},
        libipld::{Cid, IpldCodec},
    };

    #[tokio::test]
    #[serial]
    async fn get_block() -> Result<(), BlockStoreError> {
        let path = car_test_setup(2, "indexless", "carv2blockstore_get_block")?;
        let store = CarV2DiskBlockStore::load(&path).await?;
        let cid = Cid::from_str("bafy2bzaced4ueelaegfs5fqu4tzsh6ywbbpfk3cxppupmxfdhbpbhzawfw5oy")?;
        let _ = store.get_block(&cid).await?.to_vec();
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn put_block() -> Result<(), BlockStoreError> {
        let path = car_test_setup(2, "indexless", "carv2blockstore_put_block")?;
        let store = CarV2DiskBlockStore::load(&path).await?;
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store.put_block(kitty_bytes.clone(), IpldCodec::Raw).await?;

        let new_kitty_bytes = store.get_block(&kitty_cid).await?.to_vec();
        assert_eq!(kitty_bytes, new_kitty_bytes);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn from_scratch() -> Result<(), BlockStoreError> {
        let original_path = &Path::new("test")
            .join("car")
            .join("carv2_carv2blockstore_from_scratch.car");

        if original_path.exists() {
            remove_file(original_path)?;
        }

        // Create new file
        let original = CarV2DiskBlockStore::new(original_path).await?;
        // Put a block in
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = original
            .put_block(kitty_bytes.clone(), IpldCodec::Raw)
            .await?;
        // Insert root
        original.set_root(&kitty_cid).await;
        // Save
        original.to_disk().await?;

        // Reopen
        let reconstructed = CarV2DiskBlockStore::load(original_path).await?;

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
    async fn carv2blockstore() -> Result<(), BlockStoreError> {
        let car_path = &car_test_setup(2, "indexless", "blockstore")?;
        let store = &CarV2DiskBlockStore::load(car_path).await?;
        bs_retrieval_test(store)
            .await
            .map_err(|err| BlockStoreError::wnfs(Box::from(err)))?;
        bs_duplication_test(store)
            .await
            .map_err(|err| BlockStoreError::wnfs(Box::from(err)))?;
        Ok(())
    }
}
