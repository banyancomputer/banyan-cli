use crate::types::blockstore::carv2::blockstore::BlockStore;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    cmp::max,
    fs::{self, create_dir_all, metadata},
    path::{Path, PathBuf},
};
use tomb_common::types::blockstore::{car::error::CarError, rootedblockstore::RootedBlockStore};
use wnfs::{
    common::{BlockStore as WnfsBlockStore, BlockStoreError},
    libipld::{Cid, IpldCodec},
};

#[derive(Debug, PartialEq, Clone)]
pub struct MultifileBlockStore {
    pub path: PathBuf,
    pub deltas: Vec<BlockStore>,
}

impl MultifileBlockStore {
    pub fn new(dir: &Path) -> Result<Self> {
        if dir.is_file() {
            Err(CarError::Directory(dir.to_path_buf()).into())
        } else {
            // If the folder doesn't already exist
            if !dir.exists() {
                // Make it
                create_dir_all(dir)?;
            }

            let mut deltas: Vec<BlockStore> = Vec::new();
            // For each child in the directory
            for dir_entry in fs::read_dir(dir)? {
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
            let new_store = BlockStore::new(&dir.join(format!("{}.car", deltas.len() + 1)))?;
            new_store.set_root(&Cid::default());

            // If there is already a most recent delta
            if let Some(last) = deltas.last() && let Some(root) = last.get_root() {
                // Set the root in the new blockstore too
                new_store.set_root(&root);
            }

            // Append the new BlockStore to the delta list
            deltas.push(new_store);

            Ok(Self {
                path: dir.to_path_buf(),
                deltas,
            })
        }
    }
}

#[async_trait(?Send)]
impl WnfsBlockStore for MultifileBlockStore {
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

impl RootedBlockStore for MultifileBlockStore {
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

impl Serialize for MultifileBlockStore {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // If the size of the most recent delta file is less than 100 bytes
        let deltas = if let Ok(metadata) = metadata(&self.deltas.last().unwrap().path) && metadata.len() < 100 {
            // Nothing was actually written to it, dont serialize
            &self.deltas[..max(self.deltas.len() as i32 - 2, 0) as usize]
        } else {
            // Otherwise include all deltas
            &self.deltas[..]
        };

        // Serialize
        (&self.path, deltas.to_vec()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MultifileBlockStore {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        //
        let (path, mut deltas) = <(PathBuf, Vec<BlockStore>)>::deserialize(deserializer)?;
        // Create a new path
        let new_path = &path.join(format!("{}.car", deltas.len() + 1));
        let new_store = BlockStore::new(new_path).unwrap();
        new_store.set_root(&Cid::default());

        // If there is already a most recent delta
        if let Some(last) = deltas.last() && let Some(root) = last.get_root() {
            // Set the root in the new blockstore too
            new_store.set_root(&root);
        }

        deltas.push(new_store);

        Ok(Self { path, deltas })
    }
}

#[cfg(test)]
mod test {
    use crate::types::blockstore::carv2::multifile::MultifileBlockStore;
    use anyhow::Result;
    use std::{fs::remove_dir_all, path::Path};
    use wnfs::{
        common::{dagcbor, BlockStore},
        libipld::IpldCodec,
    };

    #[tokio::test]
    async fn multidelta_serialization() -> Result<()> {
        let path = &Path::new("test").join("multidelta_serialization");
        // Delete this if it exists
        if path.exists() {
            remove_dir_all(path)?;
        }

        let store = MultifileBlockStore::new(path)?;

        // Assert that there are now two delta CARs
        assert_eq!(store.deltas.len(), 1);

        let hello_kitty = "Hello Kitty!".as_bytes().to_vec();
        let hello_kitty_cid = store.put_block(hello_kitty.clone(), IpldCodec::Raw).await?;

        // Serialize the Multifile
        let cbor_store = dagcbor::encode(&store)?;
        let r1 = dagcbor::decode::<MultifileBlockStore>(&cbor_store)?;

        let goodbye_kitty = "Goodbye Kitty!".as_bytes().to_vec();
        let goodbye_kitty_cid = r1.put_block(goodbye_kitty.clone(), IpldCodec::Raw).await?;

        // Assert that there are now two delta CARs
        assert_eq!(r1.deltas.len(), 2);

        let cbor_store = dagcbor::encode(&r1)?;
        let r2 = dagcbor::decode::<MultifileBlockStore>(&cbor_store)?;

        // Assert that there are now two delta CARs
        assert_eq!(r2.deltas.len(), 3);

        // Assert that both blocks are still retrievable, despite being in separate CAR files
        assert_eq!(r2.get_block(&hello_kitty_cid).await?.to_vec(), hello_kitty);
        assert_eq!(
            r2.get_block(&goodbye_kitty_cid).await?.to_vec(),
            goodbye_kitty
        );

        Ok(())
    }
}
