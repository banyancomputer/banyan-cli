use crate::{
    blockstore::{BlockStore, RootedBlockStore},
    car::{v1::Block, v2::CarV2},
    utils::{get_read, get_read_write, get_write},
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{de::Error as DeError, Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs::File,
    path::{Path, PathBuf},
};
use wnfs::libipld::{Cid, IpldCodec};

/// CarV2DiskBlockStore implementation using File IO
#[derive(Debug, PartialEq, Clone)]
pub struct CarV2DiskBlockStore {
    /// CarV2 file path
    pub path: PathBuf,
    /// CarV2v2
    pub car: CarV2,
}

impl CarV2DiskBlockStore {
    /// Create a new CarV2v2 CarV2DiskBlockStore from a file
    pub fn new(path: &Path) -> Result<Self> {
        // If the path is a directory
        if path.is_dir() {
            Err(anyhow!("{} is a directory", path.display()))
        } else {
            // Create the file if it doesn't already exist
            if !path.exists() {
                File::create(path)?;
            }

            // If the file is already a valid CarV2v2
            if let Ok(mut file) = File::open(path)
                && let Ok(car) = CarV2::read_bytes(&mut file)
            {
                Ok(Self {
                    path: path.to_path_buf(),
                    car,
                })
            }
            // If we need to create the CarV2v2 file from scratch
            else {
                // Grab read and write
                let mut rw = get_read_write(path)?;
                // Create new
                let store = CarV2DiskBlockStore {
                    path: path.to_path_buf(),
                    car: CarV2::new(&mut rw)?,
                };
                // Return Ok
                Ok(store)
            }
        }
    }

    /// Save the CarV2 CarV2DiskBlockStore to disk
    pub fn to_disk(&self) -> Result<()> {
        self.car.write_bytes(&mut get_read_write(&self.path)?)
    }

    /// Get the size of the underlying CARv1
    pub fn data_size(&self) -> u64 {
        self.car.data_size()
    }
}

#[async_trait(?Send)]
impl BlockStore for CarV2DiskBlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        // Open the file in read-only mode
        let mut file = get_read(&self.path)?;
        // Perform the block read
        let block: Block = self.car.get_block(cid, &mut file)?;
        // Return its contents
        Ok(Cow::Owned(block.content))
    }

    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
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
            self.car.put_block(&block, &mut file)?;
            // Return Ok with block CID
            Ok(block.cid)
        }
    }
}

#[async_trait(?Send)]
impl RootedBlockStore for CarV2DiskBlockStore {
    fn set_root(&self, root: &Cid) {
        self.car.set_root(root);
        self.to_disk().expect("failed to write to disk");
    }

    fn get_root(&self) -> Option<Cid> {
        self.car.get_root()
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
        if let Ok(new_store) = Self::new(&path) {
            // Return loaded object
            Ok(new_store)
        } else {
            // Create a new CarV2 Error
            Err(DeError::custom(anyhow!("Failed to load from disk")))
        }
    }
}

#[cfg(test)]
mod test {

    use crate::blockstore::{test::*, BlockStore, CarV2DiskBlockStore, RootedBlockStore};
    use anyhow::Result;
    use serial_test::serial;
    use std::{fs::remove_file, path::Path, str::FromStr};
    use wnfs::common::blockstore::{bs_duplication_test, bs_retrieval_test};
    use wnfs::libipld::{Cid, IpldCodec};

    #[tokio::test]
    #[serial]
    async fn get_block() -> Result<()> {
        let path = car_test_setup(2, "indexless", "carv2blockstore_get_block")?;
        let store = CarV2DiskBlockStore::new(&path)?;
        let cid = Cid::from_str("bafy2bzaced4ueelaegfs5fqu4tzsh6ywbbpfk3cxppupmxfdhbpbhzawfw5oy")?;
        let _ = store.get_block(&cid).await?.to_vec();
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn put_block() -> Result<()> {
        let path = car_test_setup(2, "indexless", "carv2blockstore_put_block")?;
        let store = CarV2DiskBlockStore::new(&path)?;
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store.put_block(kitty_bytes.clone(), IpldCodec::Raw).await?;

        let new_kitty_bytes = store.get_block(&kitty_cid).await?.to_vec();
        assert_eq!(kitty_bytes, new_kitty_bytes);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn from_scratch() -> Result<()> {
        let original_path = &Path::new("test")
            .join("car")
            .join("carv2_carv2blockstore_from_scratch.car");
        if original_path.exists() {
            remove_file(original_path)?;
        }

        // Open
        let original = CarV2DiskBlockStore::new(original_path)?;
        // Put a block in
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = original
            .put_block(kitty_bytes.clone(), IpldCodec::Raw)
            .await?;
        // Insert root
        original.set_root(&kitty_cid);
        // Save
        original.to_disk()?;

        // Reopen
        let reconstructed = CarV2DiskBlockStore::new(original_path)?;

        // Ensure content is still there
        assert_eq!(kitty_cid, original.get_root().expect("no root in CAR"));
        assert_eq!(kitty_bytes, original.get_block(&kitty_cid).await?.to_vec());

        // Assert equality
        assert_eq!(original, reconstructed);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn carv2blockstore() -> Result<()> {
        let car_path = &car_test_setup(2, "indexless", "blockstore")?;
        let store = &CarV2DiskBlockStore::new(car_path)?;
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        Ok(())
    }
}
