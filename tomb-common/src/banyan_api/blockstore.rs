use crate::banyan_api::client::Client;
use crate::banyan_api::requests::staging::blocks::pull::*;
use crate::blockstore::BlockStore;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_util::StreamExt;
use std::borrow::Cow;
use std::cell::RefCell;
use wnfs::libipld::{Cid, IpldCodec};

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
    async fn put_block(&self, _bytes: Vec<u8>, _codec: IpldCodec) -> Result<Cid> {
        // TODO
        panic!("Not yet implemented");
    }

    /// Retrieves an array of bytes from the block store with given CID.
    #[allow(clippy::await_holding_refcell_ref)]
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        let mut client = self.0.borrow_mut();
        let mut stream = client
            .stream(PullBlock { cid: *cid })
            .await
            .map_err(|_| anyhow!("Failed to pull block"))?;
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }

        Ok(Cow::Owned(data))
    }
}
