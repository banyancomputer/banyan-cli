use super::{CarV2, V2_PRAGMA};
use crate::{types::blockstore::car::carv1::v1block::V1Block, utils::car};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs::{remove_file, rename, File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use wnfs::{
    common::BlockStore,
    libipld::{Cid, IpldCodec},
};

#[derive(Debug, PartialEq, Default, Clone)]
pub struct CarV2BlockStore {
    pub path: PathBuf,
    pub(crate) carv2: CarV2,
}

impl CarV2BlockStore {
    pub fn new(path: &Path) -> Result<Self> {
        // If the path is a directory
        if path.is_dir() {
            panic!("invalid path, must be file, not dir");
        }

        // Create the file if it doesn't already exist
        if !path.exists() {
            File::create(path)?;
        }

        // If the file is already a valid CARv2
        if let Ok(mut file) = File::open(path) &&
           let Ok(carv2) = CarV2::read_bytes(&mut file) {
            Ok(Self {
                path: path.to_path_buf(),
                carv2,
            })
        }
        // If we need to create the header
        else {
            println!("\nSTARTING FROM SCRATCH\n");
            // Grab read and write
            let mut w = car::get_write(path)?;
            let mut r = car::get_read(path)?;
            let mut store = CarV2BlockStore {
                path: path.to_path_buf(),
                carv2: CarV2::new(&mut r, &mut w)?
            };
            println!("\nDEFAULT CARv2 BlockStore: {:?}\n", store);
            println!("\nFINISHED CARv2 INIT: {:?}\n", store);
            // Return Ok
            Ok(store)
        }
    }

    pub fn get_all_cids(&self) -> Vec<Cid> {
        self.carv2.get_all_cids()
    }

    pub fn insert_root(&self, root: &Cid) {
        self.carv2.insert_root(root);
    }

    pub fn empty_roots(&self) {
        self.carv2.empty_roots();
    }

    pub fn get_roots(&self) -> Vec<Cid> {
        self.carv2.carv1.header.roots.borrow().clone()
    }

    pub fn to_disk(&self) -> Result<()> {
        let (tmp_car_path, mut r, mut w) = self.tmp_start()?;
        self.carv2.write_bytes(&mut r, &mut w)?;
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
impl BlockStore for CarV2BlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        // Open the file in read-only mode
        let mut file = car::get_read(&self.path)?;
        // Perform the block read
        let block: V1Block = self.carv2.get_block(cid, &mut file)?;
        // Return its contents
        Ok(Cow::Owned(block.content))
    }

    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        // Create a block with this content
        let block = V1Block::new(bytes, codec)?;
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
            self.carv2.put_block(&block, &mut file)?;
            // Return Ok with block CID
            Ok(block.cid)
        }
    }
}

// TODO implement this so the whole struct need not be encoded

impl Serialize for CarV2BlockStore {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Write to disk just in case
        self.to_disk().unwrap();
        // Serialize path
        self.path.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CarV2BlockStore {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self::new(&PathBuf::deserialize(deserializer)?).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::CarV2BlockStore;
    use anyhow::Result;
    use serial_test::serial;
    use std::{
        fs::{copy, remove_file},
        path::Path,
        str::FromStr,
    };
    use wnfs::{
        common::BlockStore,
        libipld::{Cid, IpldCodec},
    };

    #[tokio::test]
    #[serial]
    async fn get_block() -> Result<()> {
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv2-indexless.car");
        let new_path = Path::new("test").join("carv2-indexless-get.car");
        std::fs::copy(existing_path, &new_path)?;
        let store = CarV2BlockStore::new(&new_path)?;
        println!("roots: {:?}", store.get_roots());
        let cid = Cid::from_str("bafy2bzaced4ueelaegfs5fqu4tzsh6ywbbpfk3cxppupmxfdhbpbhzawfw5oy")?;
        let _ = store.get_block(&cid).await?.to_vec();
        std::fs::remove_file(new_path)?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn put_block() -> Result<()> {
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv2-indexless.car");
        let new_path = Path::new("test").join("carv2-indexless-put.car");
        std::fs::copy(existing_path, &new_path)?;

        let store = CarV2BlockStore::new(&new_path)?;

        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store.put_block(kitty_bytes.clone(), IpldCodec::Raw).await?;

        let new_kitty_bytes = store.get_block(&kitty_cid).await?.to_vec();
        assert_eq!(kitty_bytes, new_kitty_bytes);
        std::fs::remove_file(new_path)?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn from_scratch() -> Result<()> {
        let original_path = &Path::new("test").join("carv2-from-scratch.car");
        remove_file(original_path).ok();

        // Open
        let original = CarV2BlockStore::new(original_path)?;
        // Put a block in
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        // let kitty_cid = store.put_block(kitty_bytes.clone(), IpldCodec::Raw).await?;
        // Insert root
        // store.insert_root(&kitty_cid);
        // Save
        original.to_disk()?;
        println!("\npost_write store: {:?}\n", original);

        // Reopen
        let reconstructed = CarV2BlockStore::new(original_path)?;
        println!("\nreconstructed store: {:?}\n", reconstructed);

        assert_eq!(original, reconstructed);

        // assert_eq!(kitty_cid, store.get_roots()[0]);
        // assert_eq!(kitty_bytes, store.get_block(&kitty_cid).await?.to_vec());

        Ok(())
    }
}
