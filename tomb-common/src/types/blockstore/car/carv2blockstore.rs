use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    cell::RefCell,
    fs::{File, OpenOptions},
    path::{Path, PathBuf}, io::Write,
};
use wnfs::{
    common::BlockStore,
    libipld::{Cid, IpldCodec},
};

use crate::types::blockstore::car::carv2::V2_PRAGMA;

use super::{carv1blockstore::CarV1BlockStore, v2header::V2Header, carv2::CarV2};

#[derive(Debug, Serialize, Deserialize)]
pub struct CarV2BlockStore {
    pub path: PathBuf,
    pub(crate) header: RefCell<V2Header>,
    pub(crate) child: CarV1BlockStore,
}

impl CarV2BlockStore {
    pub fn new(path: &Path) -> Result<Self> {
        // Create the file if it doesn't already exist
        if !path.exists() { File::create(path)?; }
        // Open the file in reading mode
        let file = File::open(path)?;
        // If the header is already there
        if let Ok(_) = CarV2::verify_pragma(&file) {
            let header = V2Header::read_bytes(&file)?;
            println!("loaded a v2header: {:?}", header);
            Ok(Self {
                path: path.to_path_buf(),
                header: RefCell::new(header),
                child: CarV1BlockStore::new(path, Some(RefCell::new(header)))?,
            })
        }
        // If we need to create the header
        else {
            // Open the file in append mode
            let mut file = OpenOptions::new().append(true).open(path)?;
            file.write_all(&V2_PRAGMA)?;
            // Create a new header
            let new_header = V2Header {
                characteristics: 0,
                data_offset: 0,
                data_size: 0,
                index_offset: 0,
            };
            // Write the header to the file
            new_header.write_bytes(&mut file)?;
            println!("had to make a v2header: {:?}", new_header);
            // Return Ok
            Ok(Self {
                path: path.to_path_buf(),
                header: RefCell::new(new_header.clone()),
                child: CarV1BlockStore::new(path, Some(RefCell::new(new_header.clone())))?,
            })
        }
    }

    pub fn get_read(&self) -> Result<File> {
        Ok(File::open(&self.path)?)
    }
    pub fn get_write(&self) -> Result<File> {
        // Open the file in append mode
        Ok(OpenOptions::new().append(true).open(&self.path)?)
    }
}

#[async_trait(?Send)]
impl BlockStore for CarV2BlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        self.child.get_block(cid).await
    }

    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        let cid = self.child.put_block(bytes, codec).await?;
        let block = self.child.find_block(&cid)?;
        let length = block.to_bytes().len() as u64;

        let mut header = self.header.borrow_mut();
        header.data_size += length;

        Ok(cid)
    }
}

