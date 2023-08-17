use crate::types::blockstore::error::{CARIOError, SingleError::*};
use anyhow::Result;
use async_trait::async_trait;
use serde::{de::Error as DeError, ser::Error as SerError, Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs::File,
    path::{Path, PathBuf},
};
use tomb_common::{
    blockstore::TombBlockStore,
    car::v1::{block::Block, CarV1},
    utils::io::{get_read, get_read_write, get_write},
};
use wnfs::{
    common::BlockStore as WnfsBlockStore,
    libipld::{Cid, IpldCodec},
};

/// CarV1v1 BlockStore implementation using File IO
#[derive(Debug, PartialEq)]
pub struct BlockStore {
    /// CarV1 file path
    pub path: PathBuf,
    pub(crate) car: CarV1,
}

impl BlockStore {
    /// Create a new CarV1v1 BlockStore from a file
    pub fn new(path: &Path) -> Result<Self> {
        // If the path is a directory
        if path.is_dir() {
            Err(CARIOError::SingleError(Directory(path.to_path_buf())).into())
        } else {
            // Create the file if it doesn't already exist
            if !path.exists() {
                File::create(path)?;
            }

            // Open the file in reading mode
            if let Ok(mut file) = File::open(path) &&
                let Ok(car) = CarV1::read_bytes(None, &mut file) {
                Ok(Self {
                    path: path.to_path_buf(),
                    car
                })
            }
            // If we need to create the CarV1v2 file from scratch
            else {
                // Grab reader and writer
                let mut rw = get_read_write(path)?;

                // Construct new
                Ok(Self {
                    path: path.to_path_buf(),
                    car: CarV1::new(None, &mut rw)?
                })
            }
        }
    }

    /// Save the CarV1 BlockStore to disk
    pub fn to_disk(&self) -> Result<()> {
        self.car.write_bytes(&mut get_read_write(&self.path)?)
    }
}

#[async_trait(?Send)]
impl WnfsBlockStore for BlockStore {
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
        // If this needs to be appended to the CarV1v1
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
impl TombBlockStore for BlockStore {
    fn set_root(&self, root: &Cid) {
        self.car.set_root(root);
    }

    fn get_root(&self) -> Option<Cid> {
        self.car.get_root()
    }

    async fn update_block(&self, _: &Cid, _: Vec<u8>, _: IpldCodec) -> Result<Cid> {
        panic!("help!")
    }
}

impl Serialize for BlockStore {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // If we successfully save ourself to disk
        if self.to_disk().is_ok() {
            // Serialize the Path
            self.path.serialize(serializer)
        } else {
            // Create a new CarV1 Error
            Err(SerError::custom(CARIOError::SingleError(FailToSave)))
        }
    }
}

impl<'de> Deserialize<'de> for BlockStore {
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
            // Create a new CarV1 Error
            Err(DeError::custom(CARIOError::SingleError(FailToLoad(path))))
        }
    }
}

#[cfg(test)]
mod test {
    use super::BlockStore;
    use anyhow::Result;
    use serial_test::serial;
    use std::{fs::remove_file, path::Path, str::FromStr};
    use tomb_common::{blockstore::TombBlockStore, utils::tests::car_test_setup};
    use wnfs::{
        common::BlockStore as WnfsBlockStore,
        libipld::{Cid, IpldCodec},
    };

    #[tokio::test]
    #[serial]
    async fn get_block() -> Result<()> {
        let car_path = &car_test_setup(1, "basic", "get_block")?;
        let store = BlockStore::new(car_path)?;
        let cid = Cid::from_str("QmdwjhxpxzcMsR3qUuj7vUL8pbA7MgR3GAxWi2GLHjsKCT")?;
        let bytes = store.get_block(&cid).await?.to_vec();
        assert_eq!(bytes, hex::decode("122d0a240155122061be55a8e2f6b4e172338bddf184d6dbee29c98853e0a0485ecee7f27b9af0b412036361741804")?);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn put_block() -> Result<()> {
        let car_path = &car_test_setup(1, "basic", "put_block")?;
        let store = BlockStore::new(car_path)?;
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store
            .put_block(kitty_bytes.clone(), IpldCodec::DagCbor)
            .await?;
        let new_kitty_bytes = store.get_block(&kitty_cid).await?.to_vec();
        assert_eq!(kitty_bytes, new_kitty_bytes);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn set_root() -> Result<()> {
        let car_path = &car_test_setup(1, "basic", "set_root")?;
        let store = BlockStore::new(car_path)?;

        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store
            .put_block(kitty_bytes.clone(), IpldCodec::DagCbor)
            .await?;
        store.set_root(&kitty_cid);
        assert_eq!(kitty_cid, store.get_root().expect("no root in CAR"));
        assert_eq!(kitty_bytes, store.get_block(&kitty_cid).await?.to_vec());
        Ok(())
    }

    #[test]
    #[serial]
    fn to_from_disk_no_offset() -> Result<()> {
        let car_path = &car_test_setup(1, "basic", "blockstore_to_from_disk_no_offset")?;

        // Read in the car
        let original = BlockStore::new(car_path)?;
        // Write it to disk
        original.to_disk()?;

        // Read in the new car
        let reconstructed = BlockStore::new(car_path)?;

        // Assert equality
        assert_eq!(original.car.header, reconstructed.car.header);
        assert_eq!(original.car.index, reconstructed.car.index);
        assert_eq!(original, reconstructed);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn to_from_disk_with_offset() -> Result<()> {
        let car_path = &car_test_setup(1, "basic", "blockstore_to_from_disk_with_offset")?;

        // Read in the car
        let original = BlockStore::new(car_path)?;

        // Write contentt
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let cid = original
            .put_block(kitty_bytes.clone(), IpldCodec::Raw)
            .await?;
        // Insert root
        original.set_root(&cid);
        // Write BlockStore to disk
        original.to_disk()?;

        // Read in the new car
        let reconstructed = BlockStore::new(car_path)?;

        // Assert equality
        assert_eq!(original.car.header, reconstructed.car.header);
        assert_eq!(
            original.car.index.borrow().get_all_cids(),
            reconstructed.car.index.borrow().get_all_cids()
        );
        assert_eq!(original, reconstructed);

        assert_eq!(kitty_bytes, reconstructed.get_block(&cid).await?.to_vec());
        assert_eq!(
            &cid,
            reconstructed
                .car
                .header
                .roots
                .borrow()
                .clone()
                .last()
                .expect("there is no root in this CAR")
        );

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn from_scratch() -> Result<()> {
        let dir = &Path::new("test").join("car");
        if !dir.exists() {
            create_dir_all(dir)?;
        }
        let original_path = &dir.join("carv1_blockstore_from_scratch.car");

        // Remove it if its still there from previous test
        if original_path.exists() {
            remove_file(original_path)?;
        }

        // Open
        let store = BlockStore::new(original_path)?;
        // Put a block in
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store.put_block(kitty_bytes.clone(), IpldCodec::Raw).await?;
        // Insert root
        store.set_root(&kitty_cid);
        // Save
        store.to_disk()?;

        // Reopen
        let store = BlockStore::new(original_path)?;
        assert_eq!(kitty_cid, store.car.header.roots.borrow().clone()[0]);
        assert_eq!(kitty_bytes, store.get_block(&kitty_cid).await?.to_vec());

        Ok(())
    }
}
