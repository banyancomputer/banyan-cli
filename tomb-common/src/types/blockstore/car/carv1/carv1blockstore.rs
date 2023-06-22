use super::{v1block::V1Block, CarV1};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs::{remove_file, rename, File, OpenOptions},
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
};
use wnfs::{
    common::BlockStore,
    libipld::{Cid, IpldCodec},
};

#[derive(Debug, PartialEq, Default)]
pub struct CarV1BlockStore {
    pub path: PathBuf,
    pub(crate) carv1: CarV1,
    pub(crate) parent_offset: Option<u64>,
}

impl CarV1BlockStore {
    pub fn get_read(&self) -> Result<File> {
        Ok(File::open(&self.path)?)
    }
    pub fn get_write(&self) -> Result<File> {
        Ok(OpenOptions::new().append(true).open(&self.path)?)
    }

    // Create a new CARv1 BlockStore from a file
    pub fn new(path: &Path, parent_offset: Option<u64>) -> Result<Self> {
        // If the path is a directory
        if path.is_dir() {
            panic!("invalid path, must be file, not dir");
        }

        // Create the file if it doesn't already exist
        if !path.exists() {
            File::create(path)?;
        }

        // Open the file in reading mode
        if let Ok(mut file) = File::open(path) &&
            let Ok(carv1) = CarV1::read_bytes(&mut file) {
            Ok(Self {
                path: path.to_path_buf(),
                carv1,
                parent_offset,
            })
        }
        // If we need to create the header
        else {
            // Open the file in append mode
            let mut file = OpenOptions::new().append(true).open(path)?;
            // Move to start
            file.seek(SeekFrom::Start(0))?;
            // Initialize this file as a CARv1
            CarV1::initialize(&mut file)?;
            // Move back to the start of the file
            file.seek(SeekFrom::Start(0))?;
            // Return Ok
            Ok(Self {
                path: path.to_path_buf(),
                carv1: CarV1::read_bytes(&mut File::open(path)?)?,
                parent_offset,
            })
        }
    }

    pub fn get_all_cids(&self) -> Vec<Cid> {
        self.carv1.get_all_cids()
    }

    pub fn insert_root(&self, root: &Cid) {
        self.carv1.insert_root(root);
    }

    pub fn to_disk(&self) -> Result<()> {
        let (tmp_car_path, mut r, mut w) = self.tmp_start()?;
        self.carv1.write_bytes(&mut r, &mut w)?;
        self.tmp_finish(tmp_car_path)?;
        Ok(())
    }

    fn tmp_start(&self) -> Result<(PathBuf, File, File)> {
        let r = self.get_read()?;
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
impl BlockStore for CarV1BlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        // Open the file in read-only mode
        let mut file = self.get_read()?;
        // Perform the block read
        let block: V1Block = self.carv1.get_block(cid, &mut file)?;
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
            let mut file = self.get_write()?;
            // Put the block
            self.carv1.put_block(&block, &mut file)?;
            // Return Ok with block CID
            Ok(block.cid)
        }
    }
}

impl Serialize for CarV1BlockStore {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_disk().unwrap();
        self.path.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CarV1BlockStore {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self::new(&PathBuf::deserialize(deserializer)?, None).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{copy, remove_file},
        path::Path,
        str::FromStr,
    };

    use super::CarV1BlockStore;
    use anyhow::Result;
    use serial_test::serial;
    use wnfs::{
        common::BlockStore,
        libipld::{Cid, IpldCodec},
    };

    #[tokio::test]
    #[serial]
    async fn get_block() -> Result<()> {
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv1-basic.car");
        let new_path = Path::new("test").join("carv1-basic-get.car");
        std::fs::copy(existing_path, &new_path)?;

        let store = CarV1BlockStore::new(&new_path, None)?;
        let cid = Cid::from_str("QmdwjhxpxzcMsR3qUuj7vUL8pbA7MgR3GAxWi2GLHjsKCT")?;
        let bytes = store.get_block(&cid).await?.to_vec();
        assert_eq!(bytes, hex::decode("122d0a240155122061be55a8e2f6b4e172338bddf184d6dbee29c98853e0a0485ecee7f27b9af0b412036361741804")?);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn put_block() -> Result<()> {
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv1-basic.car");
        let new_path = Path::new("test").join("carv1-basic-put.car");
        copy(existing_path, &new_path)?;

        let store = CarV1BlockStore::new(&new_path, None)?;

        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store
            .put_block(kitty_bytes.clone(), IpldCodec::DagCbor)
            .await?;
        let new_kitty_bytes = store.get_block(&kitty_cid).await?.to_vec();
        assert_eq!(kitty_bytes, new_kitty_bytes);

        remove_file(new_path)?;
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn insert_root() -> Result<()> {
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv1-basic.car");
        let new_path = Path::new("test").join("carv1-basic-blockstore-insert-root.car");
        copy(existing_path, &new_path)?;

        let store = CarV1BlockStore::new(&new_path, None)?;

        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store
            .put_block(kitty_bytes.clone(), IpldCodec::DagCbor)
            .await?;

        assert_eq!(store.carv1.header.roots.borrow().clone().len(), 2);
        store.insert_root(&kitty_cid);
        assert_eq!(store.carv1.header.roots.borrow().clone().len(), 3);
        assert_eq!(kitty_bytes, store.get_block(&kitty_cid).await?.to_vec());
        remove_file(new_path)?;
        Ok(())
    }

    #[test]
    #[serial]
    fn to_from_disk_no_offset() -> Result<()> {
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv1-basic.car");
        let original_path = &Path::new("test").join("carv1-blockstore-to-from-disk-no-offset.car");
        copy(&existing_path, &original_path)?;

        // Read in the car
        let original = CarV1BlockStore::new(original_path, None)?;
        // Write it to disk
        original.to_disk()?;

        // Read in the new car
        let reconstructed = CarV1BlockStore::new(original_path, None)?;

        // Assert equality
        assert_eq!(original.carv1.header, reconstructed.carv1.header);
        assert_eq!(original.carv1.index, reconstructed.carv1.index);
        assert_eq!(original, reconstructed);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn to_from_disk_with_offset() -> Result<()> {
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv1-basic.car");
        let original_path =
            &Path::new("test").join("carv1-blockstore-to-from-disk-with-offset.car");
        copy(&existing_path, &original_path)?;

        // Read in the car
        let original = CarV1BlockStore::new(original_path, None)?;

        // Write contentt
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let cid = original
            .put_block(kitty_bytes.clone(), IpldCodec::Raw)
            .await?;
        // Insert root
        original.insert_root(&cid);

        // Write BlockStore to disk
        original.to_disk()?;

        // Read in the new car
        let reconstructed = CarV1BlockStore::new(original_path, None)?;

        // Assert equality
        assert_eq!(original.carv1.header, reconstructed.carv1.header);
        assert_eq!(original.carv1.index, reconstructed.carv1.index);
        assert_eq!(original, reconstructed);

        assert_eq!(kitty_bytes, reconstructed.get_block(&cid).await?.to_vec());
        assert_eq!(
            &cid,
            reconstructed
                .carv1
                .header
                .roots
                .borrow()
                .clone()
                .last()
                .unwrap()
        );

        Ok(())
    }
}
