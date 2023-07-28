use crate::types::blockstore::{
    carv2,
    error::{CARIOError, MultiError::*},
};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs::{self, create_dir_all},
    path::{Path, PathBuf},
};
use tomb_common::types::blockstore::{tombblockstore::TombBlockStore, car::carv2::index::indexbucket::IndexBucket};
use wnfs::{
    common::{BlockStore as WnfsBlockStore, BlockStoreError},
    libipld::{Cid, IpldCodec},
};

/// CARv2 BlockStore across multiple CAR files using File IO
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct BlockStore {
    /// CAR directory path
    pub path: PathBuf,
    /// CARv2 BlockStores inside this BlockStore
    pub deltas: Vec<carv2::BlockStore>,
}

impl BlockStore {
    /// Create a new CARv2 MultifileBlockStore from a directory
    pub fn new(dir: &Path) -> Result<Self> {
        if dir.is_file() {
            Err(CARIOError::MultiError(File(dir.to_path_buf())).into())
        } else {
            // If the folder doesn't already exist
            if !dir.exists() {
                // Make it
                create_dir_all(dir)?;
            }

            let mut deltas: Vec<carv2::BlockStore> = Vec::new();
            // For each child in the directory
            for dir_entry in fs::read_dir(dir)? {
                // If the dir entry is valid, the file is a .car, and a BlockStore can be read from it
                if let Ok(entry) = dir_entry &&
                   entry.file_name().to_str().unwrap().contains(".car") &&
                   let Ok(car) = carv2::BlockStore::new(&entry.path()) {
                    // Push this to the vec
                    deltas.push(car);
                }
            }

            // Ok
            Ok(Self {
                path: dir.to_path_buf(),
                deltas,
            })
        }
    }

    /// Add a new delta file / CAR file
    pub(crate) fn add_delta(&mut self) -> Result<()> {
        // Create a new delta for writing
        let new_store =
            carv2::BlockStore::new(&self.path.join(format!("{}.car", self.deltas.len() + 1)))?;
        new_store.set_root(&Cid::default());

        // If there is already a most recent delta
        if let Some(last) = self.deltas.last() && let Some(root) = last.get_root() {
            // Set the root in the new blockstore too
            new_store.set_root(&root);
        }

        // Add the new store
        self.deltas.push(new_store);

        // Ok
        Ok(())
    }
}

#[async_trait(?Send)]
impl WnfsBlockStore for BlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        // Iterate in reverse order
        for store in self.deltas.iter().rev() {
            println!("searching through {}", store.path.display());
            // If block is retrieved
            if let Ok(data) = store.get_block(cid).await {
                // Ok
                return Ok(data);
            }
        }

        // We didn't find the CID in any BlockStore
        Err(BlockStoreError::CIDNotFound(*cid).into())
    }

    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        // If there is a delta
        if let Some(current_delta) = self.deltas.last() {
            let cid = current_delta.put_block(bytes, codec).await?;
            println!("putting {} in {}", cid, current_delta.path.display());
            Ok(cid)
        } else {
            Err(BlockStoreError::LockPoisoned.into())
        }
    }
}

#[async_trait(?Send)]
impl TombBlockStore for BlockStore {
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

    async fn update_content(&self, cid: &Cid, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        // Iterate in reverse order
        for store in self.deltas.iter().rev() {
            // Bind to avoid awaiting
            let index = store.car.car.index.borrow().clone();
            // If this store has the data we are replacing
            if index.get_offset(cid).is_some() {
                // Update the content in this store and return new cid
                return store.update_content(cid, bytes, codec).await;
            }
        }

        // We didn't find the CID in any BlockStore
        Err(BlockStoreError::CIDNotFound(*cid).into())
    }
}

#[cfg(test)]
mod test {
    use crate::types::blockstore::multi::BlockStore;
    use anyhow::Result;
    use serial_test::serial;
    use std::{fs::remove_dir_all, path::Path};
    use wnfs::{common::BlockStore as WnfsBlockStore, libipld::IpldCodec};

    #[tokio::test]
    #[serial]
    async fn multidelta() -> Result<()> {
        let path = &Path::new("test").join("multidelta");
        // Delete this if it exists
        if path.exists() {
            remove_dir_all(path)?;
        }

        let mut store = BlockStore::new(path)?;

        // Create a new delta
        store.add_delta()?;

        // Assert that there are now two delta CARs
        assert_eq!(store.deltas.len(), 1);

        let hello_kitty = "Hello Kitty!".as_bytes().to_vec();
        let hello_kitty_cid = store.put_block(hello_kitty.clone(), IpldCodec::Raw).await?;

        // Create a new delta
        store.add_delta()?;
        // Assert that there are now two delta CARs
        assert_eq!(store.deltas.len(), 2);

        let goodbye_kitty = "Goodbye Kitty!".as_bytes().to_vec();
        let goodbye_kitty_cid = store
            .put_block(goodbye_kitty.clone(), IpldCodec::Raw)
            .await?;

        // Assert that both blocks are still retrievable, despite being in separate CAR files
        assert_eq!(
            store.get_block(&hello_kitty_cid).await?.to_vec(),
            hello_kitty
        );
        assert_eq!(
            store.get_block(&goodbye_kitty_cid).await?.to_vec(),
            goodbye_kitty
        );

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn unidelta() -> Result<()> {
        let path = &Path::new("test").join("unidelta");
        // Delete this if it exists
        if path.exists() {
            remove_dir_all(path)?;
        }

        let mut store = BlockStore::new(path)?;

        // Create a new delta
        store.add_delta()?;
        // Assert that there is one CAR
        assert_eq!(store.deltas.len(), 1);

        let hello_kitty = "Hello Kitty!".as_bytes().to_vec();
        let hello_kitty_cid = store.put_block(hello_kitty.clone(), IpldCodec::Raw).await?;

        let goodbye_kitty = "Goodbye Kitty!".as_bytes().to_vec();
        let goodbye_kitty_cid = store
            .put_block(goodbye_kitty.clone(), IpldCodec::Raw)
            .await?;

        // Assert that both blocks are still retrievable, despite being in separate CAR files
        assert_eq!(
            store.get_block(&hello_kitty_cid).await?.to_vec(),
            hello_kitty
        );
        assert_eq!(
            store.get_block(&goodbye_kitty_cid).await?.to_vec(),
            goodbye_kitty
        );

        Ok(())
    }
}
