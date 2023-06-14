use super::{v1block::V1Block, CarV1};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
};
use wnfs::{
    common::BlockStore,
    libipld::{Cid, IpldCodec},
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
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

#[cfg(test)]
mod tests {
    use std::{path::Path, str::FromStr};

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
        std::fs::copy(existing_path, &new_path)?;

        let store = CarV1BlockStore::new(&new_path, None)?;

        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store
            .put_block(kitty_bytes.clone(), IpldCodec::DagCbor)
            .await?;
        let new_kitty_bytes = store.get_block(&kitty_cid).await?.to_vec();
        assert_eq!(kitty_bytes, new_kitty_bytes);

        Ok(())
    }
}
