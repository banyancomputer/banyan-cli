use crate::banyan_common::blockstore::BlockStore;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};
use wnfs::libipld::{Cid, IpldCodec};

/// A disk-based blockstore that you can mutate.
#[derive(Debug, Serialize, Deserialize)]
pub struct DiskBlockStore {
    /// The path at which the BlockStore is stored
    pub path: PathBuf,
}

// -------------------------------------------------------------------------------------------------
// Implementations
// -------------------------------------------------------------------------------------------------

impl DiskBlockStore {
    /// Creates a new disk block store.
    pub fn new(path: &Path) -> Self {
        // Return the new DiskBlockStore
        Self {
            path: path.to_path_buf(),
        }
    }
}

impl Clone for DiskBlockStore {
    fn clone(&self) -> Self {
        Self::new(&self.path)
    }
}

#[async_trait(?Send)]
impl BlockStore for DiskBlockStore {
    /// Stores an array of bytes in the block store.
    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        // If the parent directory doesn't already exist
        if !self.path.exists() {
            // Create the directories required to store the blocks
            std::fs::create_dir_all(&self.path).expect("failed to create all dirs");
        }

        // Try to build the CID from the bytes and codec
        let cid = self.create_cid(&bytes, codec)?;
        let file_path = self.path.join(cid.to_string());

        // If this file has not already been written to disk
        if !file_path.exists() {
            // Create the file at the specified path
            let mut file = std::fs::File::create(file_path)?;
            // Write the bytes to disk at the File location
            std::io::Write::write_all(&mut file, &bytes)?;
        }

        // Return Ok status with the generated CID
        Ok(cid)
    }

    /// Retrieves an array of bytes from the block store with given CID.
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        // Get the bytes from disk, using the given CID as the filename
        let mut file = std::fs::File::open(self.path.join(cid.to_string()))?;
        // Create a mutable vector of bytes
        let mut bytes: Vec<u8> = Vec::new();
        // Read the bytes into that
        std::io::Read::read_to_end(&mut file, &mut bytes)?;
        // Return Ok status with the bytes
        return Ok(Cow::Owned(bytes));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;
    use std::{fs::create_dir_all, path::PathBuf};
    use wnfs::common::blockstore::{bs_duplication_test, bs_retrieval_test, bs_serialization_test};

    #[tokio::test]
    async fn diskblockstore() -> Result<()> {
        let dir = &PathBuf::from("test").join("diskblockstore");
        create_dir_all(dir)?;
        let store = &DiskBlockStore::new(dir);
        bs_retrieval_test(store).await?;
        bs_duplication_test(store).await?;
        bs_serialization_test(store).await
    }
}
