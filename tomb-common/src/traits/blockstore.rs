use crate::{keys::manager::Manager, utils::serialize::{load_all, init_all}};
use anyhow::Result;
use async_trait::async_trait;
use std::rc::Rc;
use tomb_crypt::prelude::*;
use wnfs::{
    common::blockstore::BlockStore,
    libipld::{Cid, IpldCodec},
    private::{PrivateDirectory, PrivateForest},
};

// TODO: Use better error types
/// Wrap a BlockStore with additional functionality to expose a TombFs over it
#[async_trait(?Send)]
pub trait TombBlockStore: BlockStore {
    /// Get the root CID
    fn get_root(&self) -> Option<Cid>;
    /// Set the root CID
    fn set_root(&self, root: &Cid);
    /// Update the bytes of a block in-place
    async fn update_block(&self, cid: &Cid, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid>;

    /// Initialize a TombFs over a BlockStore
    async fn init(
        &self,
        key: &EcEncryptionKey,
    ) -> Result<(
        Rc<PrivateForest>,
        Rc<PrivateForest>,
        Rc<PrivateDirectory>,
        Manager,
        Cid,
    )> {
        let components = init_all(self, key).await?;
        let (metadata_forest, content_forest, dir, manager, root) = components;
        Ok((metadata_forest, content_forest, dir, manager, root))
    }

    /// Unlock a TombFs over a BlockStore
    async fn unlock(
        &self,
        key: &EcEncryptionKey,
    ) -> Result<(
        Rc<PrivateForest>,
        Rc<PrivateForest>,
        Rc<PrivateDirectory>,
        Manager,
        Cid,
    )> {
        let components = load_all(key, self).await?;
        let (metadata_forest, content_forest, dir, manager, root) = components;
        Ok((metadata_forest, content_forest, dir, manager, root))
    }
}
