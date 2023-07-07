use crate::types::blockstore::carv2::blockstore::BlockStore;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fs, path::PathBuf};
use tomb_common::types::blockstore::car::{carblockstore::CarBlockStore, error::CarError};
use wnfs::{
    common::{BlockStore as WnfsBlockStore, BlockStoreError},
    libipld::{Cid, IpldCodec},
};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Multifile {
    pub dir: PathBuf,
    pub deltas: Vec<BlockStore>,
}

impl Multifile {
    pub fn new(dir: PathBuf) -> Result<Self> {
        if dir.is_file() {
            Err(CarError::Directory(dir).into())
        } else {
            let mut deltas: Vec<BlockStore> = Vec::new();
            // For each child in the directory
            for dir_entry in fs::read_dir(&dir)? {
                // If the dir entry is valid, the file is a .car, and a BlockStore can be read from it
                if let Ok(entry) = dir_entry &&
                   entry.file_name().to_str().unwrap().contains(".car") &&
                   let Ok(car) = BlockStore::new(&entry.path()) {
                    // Push this to the vec
                    deltas.push(car);
                }
            }

            println!(
                "there are {} existing deltas in this multifile blockstore",
                deltas.len()
            );

            // Create a new delta for writing
            deltas.push(BlockStore::new(&dir.join("new.car"))?);

            Ok(Self { dir, deltas })
        }
    }
}

#[async_trait(?Send)]
impl WnfsBlockStore for Multifile {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        // Move backwards, starting with most recent delta
        for store in self.deltas.iter().rev() {
            if let Ok(data) = store.get_block(cid).await {
                return Ok(data);
            }
        }

        Err(BlockStoreError::CIDNotFound(*cid).into())
    }

    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        if let Some(current_delta) = self.deltas.last() {
            current_delta.put_block(bytes, codec).await
        } else {
            Err(BlockStoreError::LockPoisoned.into())
        }
    }
}

impl CarBlockStore for Multifile {
    fn get_root(&self) -> Option<Cid> {
        if let Some(car) = self.deltas.last() {
            car.get_root()
        } else {
            None
        }
    }

    fn set_root(&self, root: &Cid) {
        if let Some(car) = self.deltas.last() {
            car.set_root(root)
        }
    }
}
