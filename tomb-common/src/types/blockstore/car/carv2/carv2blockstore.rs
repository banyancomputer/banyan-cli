use super::CarV2;
use crate::types::blockstore::car::carv1::v1block::V1Block;
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct CarV2BlockStore {
    pub path: PathBuf,
    pub(crate) carv2: CarV2,
}

impl CarV2BlockStore {
    pub fn get_read(&self) -> Result<File> {
        Ok(File::open(&self.path)?)
    }
    pub fn get_write(&self) -> Result<File> {
        Ok(OpenOptions::new().append(true).open(&self.path)?)
    }

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
            // Open the file in append mode
            let mut file = OpenOptions::new().append(true).open(path)?;
            // Move to start
            file.seek(SeekFrom::Start(0))?;
            // Initialize
            CarV2::initialize(&mut file)?;
            // Move back to the start of the file
            file.seek(SeekFrom::Start(0))?;
            // Return Ok
            Ok(Self {
                path: path.to_path_buf(),
                carv2: CarV2::read_bytes(&mut File::open(path)?)?,
            })
        }
    }

    pub fn get_all_cids(&self) -> Vec<Cid> {
        self.carv2.get_all_cids()
    }

    pub fn insert_root(&self, root: &Cid) -> Result<()> {
        let mut r = self.get_read()?;
        let tmp_file_name = format!(
            "{}_tmp.car",
            self.path.file_name().unwrap().to_str().unwrap()
        );
        let tmp_car_path = self.path.parent().unwrap().join(tmp_file_name);
        let mut w = File::create(&tmp_car_path)?;
        self.carv2.insert_root(root, &mut r, &mut w)?;
        remove_file(&self.path)?;
        rename(tmp_car_path, &self.path)?;
        Ok(())
    }
}

#[async_trait(?Send)]
impl BlockStore for CarV2BlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        // Open the file in read-only mode
        let mut file = self.get_read()?;
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
            let mut file = self.get_write()?;
            // Put the block
            self.carv2.put_block(&block, &mut file)?;
            // Return Ok with block CID
            Ok(block.cid)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::blockstore::car::{carv1::v1block::V1Block, carv2::v2header::V2Header};

    use super::CarV2BlockStore;
    use anyhow::Result;
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
    async fn get_block() -> Result<()> {
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv2-basic.car");
        let new_path = Path::new("test").join("carv2-basic-get.car");
        std::fs::copy(existing_path, &new_path)?;
        let store = CarV2BlockStore::new(&new_path)?;

        println!("carv2: {:?}", store.carv2);

        let cid = Cid::from_str("QmfEoLyB5NndqeKieExd1rtJzTduQUPEV8TwAYcUiy3H5Z")?;
        let bytes = store.get_block(&cid).await?.to_vec();
        assert_eq!(bytes, hex::decode("122d0a221220d9c0d5376d26f1931f7ad52d7acc00fc1090d2edb0808bf61eeb0a152826f6261204f09f8da418a401")?);
        std::fs::remove_file(new_path)?;
        Ok(())
    }

    #[tokio::test]
    async fn put_block() -> Result<()> {
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv2-basic.car");
        let new_path = Path::new("test").join("carv2-basic-put.car");
        std::fs::copy(existing_path, &new_path)?;

        let store = CarV2BlockStore::new(&new_path)?;

        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store
            .put_block(kitty_bytes.clone(), IpldCodec::DagCbor)
            .await?;
        let new_kitty_bytes = store.get_block(&kitty_cid).await?.to_vec();
        assert_eq!(kitty_bytes, new_kitty_bytes);
        std::fs::remove_file(new_path)?;
        Ok(())
    }

    #[tokio::test]
    async fn update_header() -> Result<()> {
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv2-basic.car");
        let new_path = Path::new("test").join("carv2-basic-header.car");
        std::fs::copy(existing_path, &new_path)?;

        let store = CarV2BlockStore::new(&new_path)?;

        // Extract original header
        let original_header = store.carv2.header.borrow().clone();

        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        // Store
        store.put_block(kitty_bytes.clone(), IpldCodec::Raw).await?;

        // Extract updated header
        let new_header = store.carv2.header.borrow().clone();
        // Assert that the header is no longer default
        assert_ne!(new_header, V2Header::default());

        let mut kitty_buf: Vec<u8> = Vec::new();
        // Turn the kitty bytes into
        V1Block::new(kitty_bytes, IpldCodec::Raw)?.write_bytes(&mut kitty_buf)?;
        // Assert there are actual bytes written
        assert_ne!(kitty_buf.len(), 0);
        // Assert that the bytes of the V1Block are the total data size
        assert_eq!(
            new_header.data_size,
            original_header.data_size + kitty_buf.len() as u64
        );
        // Teardown
        std::fs::remove_file(new_path)?;
        Ok(())
    }

    #[tokio::test]
    async fn insert_root() -> Result<()> {
        let fixture_path = Path::new("car-fixtures");
        let existing_path = fixture_path.join("carv2-basic.car");
        let new_path = Path::new("test").join("carv2-basic-blockstore-insert-root.car");
        copy(existing_path, &new_path)?;

        let store = CarV2BlockStore::new(&new_path)?;
        let kitty_bytes = "Hello Kitty!".as_bytes().to_vec();
        let kitty_cid = store
            .put_block(kitty_bytes.clone(), IpldCodec::DagCbor)
            .await?;

        assert_eq!(store.carv2.carv1.header.roots.borrow().clone().len(), 1);
        let original_data_size = store.carv2.header.borrow().data_size;
        store.insert_root(&kitty_cid)?;
        let new_data_size = store.carv2.header.borrow().data_size;
        assert_ne!(original_data_size, new_data_size);
        assert_eq!(store.carv2.carv1.header.roots.borrow().clone().len(), 2);
        assert_eq!(kitty_bytes, store.get_block(&kitty_cid).await?.to_vec());
        remove_file(new_path)?;
        Ok(())
    }
}
