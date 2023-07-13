use crate::{
    types::blockstore::error::{CARIOError, SingleError::*},
    utils::car,
};
use anyhow::Result;
use async_trait::async_trait;
use serde::{de::Error as DeError, ser::Error as SerError, Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs::{remove_file, rename, File},
    io::Seek,
    path::{Path, PathBuf},
};
use tomb_common::types::blockstore::{
    car::{carv1::block::Block, carv2::CAR},
    tombblockstore::TombBlockStore,
};
use wnfs::{
    common::BlockStore as WnfsBlockStore,
    libipld::{Cid, IpldCodec},
};

/// CARv2 BlockStore implementation using File IO
#[derive(Debug, PartialEq, Clone)]
pub struct BlockStore {
    /// CAR file path
    pub path: PathBuf,
    /// CARv2
    pub car: CAR,
}

impl BlockStore {
    /// Create a new CARv2 BlockStore from a file
    pub fn new(path: &Path) -> Result<Self> {
        // If the path is a directory
        if path.is_dir() {
            Err(CARIOError::SingleError(Directory(path.to_path_buf())).into())
        } else {
            // Create the file if it doesn't already exist
            if !path.exists() {
                File::create(path)?;
            }

            // If the file is already a valid CARv2
            if let Ok(mut file) = File::open(path) &&
            let Ok(car) = CAR::read_bytes(&mut file) {
                Ok(Self {
                    path: path.to_path_buf(),
                    car,
                })
            }
            // If we need to create the CARv2 file from scratch
            else {
                // Grab read and write
                let mut w = car::get_write(path)?;
                let mut r = car::get_read(path)?;
                // Create new 
                let store = BlockStore {
                    path: path.to_path_buf(),
                    car: CAR::new(&mut r, &mut w)?
                };
                // Return Ok
                Ok(store)
            }
        }
    }

    /// Save the CAR BlockStore to disk
    pub fn to_disk(&self) -> Result<()> {
        let (tmp_car_path, mut r, mut w) = self.tmp_start()?;
        self.car.write_bytes(&mut r, &mut w)?;
        self.tmp_finish(tmp_car_path)?;
        Ok(())
    }

    fn tmp_start(&self) -> Result<(PathBuf, File, File)> {
        let r = car::get_read(&self.path)?;
        let tmp_file_name = format!(
            "{}_tmp.car",
            self.path.file_name().unwrap().to_str().unwrap()
        );
        let tmp_car_path = self.path.parent().unwrap().join(tmp_file_name);
        let w = File::create(&tmp_car_path)?;
        Ok((tmp_car_path, r, w))
    }

    fn tmp_finish(&self, tmp_car_path: PathBuf) -> Result<()> {
        remove_file(&self.path)?;
        rename(tmp_car_path, &self.path)?;
        Ok(())
    }
}

#[async_trait(?Send)]
impl WnfsBlockStore for BlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        // Open the file in read-only mode
        let mut file = car::get_read(&self.path)?;
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
        // If this needs to be appended to the CARv1
        else {
            // Open the file in append mode
            let mut file = car::get_write(&self.path)?;
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

    async fn update_content(&self, cid: &Cid, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        // Open the file in read-only mode
        let mut read_file = car::get_read(&self.path)?;
        // Perform the block read
        let block: Block = self.car.get_block(cid, &mut read_file)?;
        // // Create the new block
        let new_block = Block::new(bytes, codec)?;
        // Assert that the new version of the block is of the correct length
        assert_eq!(block.content.len(), new_block.content.len());
        // Determine where the block was read from
        let mut index = self.car.car.index.borrow_mut();
        let block_start = index.get_offset(&block.cid)?;
        // Remove existing offset
        index.map.remove(&block.cid);
        index.map.insert(new_block.cid, block_start);
        // Grab writer
        let mut write_file = car::get_write(&self.path)?;
        // Move to the right position
        write_file.seek(std::io::SeekFrom::Start(block_start))?;
        // Overwrite the block at this position
        new_block.write_bytes(&mut write_file)?;
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
            // Create a new CAR Error
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
            // Create a new CAR Error
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
    use tomb_common::{types::blockstore::tombblockstore::TombBlockStore, utils::test::car_setup};
    use wnfs::{
        common::BlockStore as WnfsBlockStore,
        libipld::{Cid, IpldCodec},
    };

    #[tokio::test]
    #[serial]
    async fn get_block() -> Result<()> {
        let path = car_setup(2, "indexless", "carv2blockstore_get_block")?;
        let store = BlockStore::new(&path)?;
        let cid = Cid::from_str("bafy2bzaced4ueelaegfs5fqu4tzsh6ywbbpfk3cxppupmxfdhbpbhzawfw5oy")?;
        let _ = store.get_block(&cid).await?.to_vec();
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn put_block() -> Result<()> {
        let path = car_setup(2, "indexless", "carv2blockstore_put_block")?;
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
        remove_file(original_path).ok();

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
        assert_eq!(kitty_cid, original.get_root().unwrap());
        assert_eq!(kitty_bytes, original.get_block(&kitty_cid).await?.to_vec());

        // Assert equality
        assert_eq!(original, reconstructed);
        Ok(())
    }
}
