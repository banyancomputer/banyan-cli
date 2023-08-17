use crate::types::blockstore::error::{CARIOError, SingleError::*};
use anyhow::Result;
use async_trait::async_trait;
use serde::{de::Error as DeError, ser::Error as SerError, Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs::File,
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
};
use tomb_common::{
        blockstore::{
            car::{
                Streamable,
                v1::block::Block,
                v2::{index::indexable::Indexable, CarV2},
            },
            TombBlockStore,
        },
    utils::io::{get_read, get_read_write, get_write},
};
use wnfs::{
    common::{BlockStore as WnfsBlockStore, BlockStoreError},
    libipld::{Cid, IpldCodec},
};

/// CarV2v2 BlockStore implementation using File IO
#[derive(Debug, PartialEq, Clone)]
pub struct BlockStore {
    /// CarV2 file path
    pub path: PathBuf,
    /// CarV2v2
    pub car: CarV2,
}

impl BlockStore {
    /// Create a new CarV2v2 BlockStore from a file
    pub fn new(path: &Path) -> Result<Self> {
        // If the path is a directory
        if path.is_dir() {
            Err(CARIOError::SingleError(Directory(path.to_path_buf())).into())
        } else {
            // Create the file if it doesn't already exist
            if !path.exists() {
                File::create(path)?;
            }

            // If the file is already a valid CarV2v2
            if let Ok(mut file) = File::open(path) &&
            let Ok(car) = CarV2::read_bytes(&mut file) {
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
                let store = BlockStore {
                    path: path.to_path_buf(),
                    car: CarV2::new(&mut rw)?
                };
                // Return Ok
                Ok(store)
            }
        }
    }

    /// Save the CarV2 BlockStore to disk
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
impl TombBlockStore for BlockStore {
    fn set_root(&self, root: &Cid) {
        self.car.set_root(root);
    }

    fn get_root(&self) -> Option<Cid> {
        self.car.get_root()
    }

    async fn update_block(&self, cid: &Cid, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        // Open the file in read-only mode
        let mut rw = get_read_write(&self.path)?;
        // Perform the block read
        let block: Block = self.car.get_block(cid, &mut rw)?;
        // // Create the new block
        let new_block = Block::new(bytes, codec)?;
        // Assert that the new version of the block is of the correct length
        assert_eq!(block.content.len(), new_block.content.len());
        // Determine where the block was read from
        let mut index = self.car.car.index.borrow_mut();
        let block_start = index
            .get_offset(&block.cid)
            .ok_or(BlockStoreError::CIDNotFound(block.cid))?;
        // Remove existing offset
        // TODO remove old cid
        // index.map.remove(&block.cid);
        index.insert_offset(&new_block.cid, block_start);
        // Move to the right position
        rw.seek(SeekFrom::Start(block_start))?;
        // Overwrite the block at this position
        new_block.write_bytes(&mut rw)?;
        // Ok
        Ok(new_block.cid)
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
            // Create a new CarV2 Error
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
            // Create a new CarV2 Error
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
        let path = car_test_setup(2, "indexless", "carv2blockstore_get_block")?;
        let store = BlockStore::new(&path)?;
        let cid = Cid::from_str("bafy2bzaced4ueelaegfs5fqu4tzsh6ywbbpfk3cxppupmxfdhbpbhzawfw5oy")?;
        let _ = store.get_block(&cid).await?.to_vec();
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn put_block() -> Result<()> {
        let path = car_test_setup(2, "indexless", "carv2blockstore_put_block")?;
        let store = BlockStore::new(&path)?;
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store.put_block(kitty_bytes.clone(), IpldCodec::Raw).await?;

        let new_kitty_bytes = store.get_block(&kitty_cid).await?.to_vec();
        assert_eq!(kitty_bytes, new_kitty_bytes);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn from_scratch() -> Result<()> {
        let original_path = &Path::new("test")
            .join("car")
            .join("carv2_carv2blockstore_from_scratch.car");
        if original_path.exists() {
            remove_file(original_path)?;
        }

        // Open
        let original = BlockStore::new(original_path)?;
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
        let reconstructed = BlockStore::new(original_path)?;

        // Ensure content is still there
        assert_eq!(kitty_cid, original.get_root().expect("no root in CAR"));
        assert_eq!(kitty_bytes, original.get_block(&kitty_cid).await?.to_vec());

        // Assert equality
        assert_eq!(original, reconstructed);
        Ok(())
    }
}
