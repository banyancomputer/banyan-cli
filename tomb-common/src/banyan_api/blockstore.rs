use crate::banyan_api::client::Client;
use crate::banyan_api::requests::staging::blocks::pull::*;
use crate::blockstore::BlockStore;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::StreamExt;
use std::cell::RefCell;
use libipld::Cid;

/// A network-based BlockStore designed to interface with a Kubo node or an API which mirrors it
#[derive(Debug, Clone)]
pub struct BanyanApiBlockStore(RefCell<Client>);

impl From<Client> for BanyanApiBlockStore {
    fn from(client: Client) -> Self {
        Self(RefCell::new(client))
    }
}

#[async_trait(?Send)]
impl BlockStore for BanyanApiBlockStore {
    /// Stores an array of bytes in the block store.
    async fn put_block(&self, _bytes: impl Into<Bytes>, _codec: u64) -> Result<Cid> {
        // TODO
        panic!("Not yet implemented");
    }

    /// Retrieves an array of bytes from the block store with given CID.
    #[allow(clippy::await_holding_refcell_ref)]
    async fn get_block(&self, cid: &Cid) -> Result<Bytes> {
        let mut client = self.0.borrow_mut();
        let mut stream = client
            .stream(PullBlock { cid: *cid })
            .await
            .map_err(|_| anyhow!("Failed to pull block"))?;
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }
        Ok(Bytes::from(data))
    }
}
