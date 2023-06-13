use super::{v1block::V1Block, v1header::V1Header};
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
    common::{BlockStore, BlockStoreError},
    libipld::{Cid, IpldCodec},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct CarV1BlockStore {
    path: PathBuf,
    header: V1Header,
    data_offset: u64,
}

impl CarV1BlockStore {
    pub fn get_read(&self) -> Result<File> {
        Ok(File::open(&self.path)?)
    }
    pub fn get_write(&self) -> Result<File> {
        // Open the file in append mode
        Ok(OpenOptions::new().append(true).open(&self.path)?)
    }

    pub fn new(path: &Path) -> Result<Self> {
        // Attempt to read the header
        let mut file = File::open(path)?;
        if let Ok(header) = V1Header::read_bytes(&file) {
            let data_offset = file.stream_position()?;
            Ok(Self {
                path: path.to_path_buf(),
                header,
                data_offset,
            })
        } else {
            // Open the file in append mode
            let mut file = OpenOptions::new().append(true).open(path)?;

            // Create a new header
            let new_header = V1Header {
                version: 1,
                roots: None,
            };

            // Write the header to the file
            new_header.write_bytes(&mut file)?;
            let data_offset = file.stream_position()?;

            Ok(Self {
                path: path.to_path_buf(),
                header: new_header,
                data_offset,
            })
        }
    }

    // Find a block in the CARv1
    fn find_block(&self, cid: &Cid) -> Result<V1Block> {
        let mut file = self.get_read()?;
        // Move to the data offset
        file.seek(SeekFrom::Start(self.data_offset))?;
        // While we're able to peek varints and CIDs
        while let Ok((varint, found_cid)) = V1Block::start_read(&file) {
            // If the CID matches
            if &found_cid == cid {
                // Finish the Block read and return the block
                return V1Block::finish_read(varint, found_cid, file);
            }
            // Otherwise
            else {
                // Skip the rest of the block
                file.seek(SeekFrom::Current(
                    varint as i64 - found_cid.to_bytes().len() as i64,
                ))?;
            }
        }

        // Throw CID not found error if we made it this far
        Err(anyhow::Error::new(BlockStoreError::CIDNotFound(*cid)))
    }
}

#[async_trait(?Send)]
impl BlockStore for CarV1BlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        // If we can find the block with this CID
        let block = self.find_block(cid)?;
        // Return its contents
        Ok(Cow::Owned(block.content))
    }

    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        // Create a block with this content
        let block = V1Block::new(bytes, codec)?;
        // If this CID already exists in the store
        if let Ok(block) = self.find_block(&block.cid) {
            // Return OK
            Ok(block.cid)
        }
        // If this needs to be appended to the CARv1
        else {
            // Open the file in append mode
            let mut file = self.get_write()?;
            // Move to the end of this file
            file.seek(SeekFrom::End(0))?;
            // Create a new V1Block from the bytes and write them to the end of the file
            block.write_bytes(file)?;
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
        let existing_path = Path::new("carv1-basic.car");
        let new_path = Path::new("carv1-basic-get.car");
        std::fs::copy(existing_path, new_path)?;

        let store = CarV1BlockStore::new(new_path)?;

        let cid = Cid::from_str("QmdwjhxpxzcMsR3qUuj7vUL8pbA7MgR3GAxWi2GLHjsKCT")?;
        let bytes = store.get_block(&cid).await?.to_vec();
        assert_eq!(bytes, hex::decode("122d0a240155122061be55a8e2f6b4e172338bddf184d6dbee29c98853e0a0485ecee7f27b9af0b412036361741804")?);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn put_block() -> Result<()> {
        let existing_path = Path::new("carv1-basic.car");
        let new_path = Path::new("carv1-basic-put.car");
        std::fs::copy(existing_path, new_path)?;

        let store = CarV1BlockStore::new(new_path)?;

        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store
            .put_block(kitty_bytes.clone(), IpldCodec::DagCbor)
            .await?;
        let new_kitty_bytes = store.get_block(&kitty_cid).await?.to_vec();
        assert_eq!(kitty_bytes, new_kitty_bytes);

        Ok(())
    }
}