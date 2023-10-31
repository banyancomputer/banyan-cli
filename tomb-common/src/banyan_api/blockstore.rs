use crate::banyan_api::client::Client;
use crate::banyan_api::requests::staging::blocks::pull::*;
use crate::blockstore::BlockStore;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Url;
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{BTreeSet, HashMap},
};
use wnfs::libipld::{Cid, IpldCodec};

use super::requests::core::blocks::locate::LocationRequest;

/// A network-based BlockStore designed to interface with a Kubo node or an API which mirrors it

#[derive(Debug, Clone)]
pub struct BanyanApiBlockStore {
    client: Client,
    /// Known remote endpoints of Blocks
    block_locations: RefCell<HashMap<String, Vec<String>>>,
}

impl From<Client> for BanyanApiBlockStore {
    fn from(client: Client) -> Self {
        Self {
            client,
            block_locations: RefCell::new(HashMap::new()),
        }
    }
}

impl BanyanApiBlockStore {
    /// Find the locations associated with a set of CIDs for fast querying on lookup
    pub async fn find_cids(&self, cids: BTreeSet<Cid>) -> Result<()> {
        let request = LocationRequest { cids };
        let mut client = self.client.clone();
        let response = client.call(request).await?;
        let mut block_locations = self.block_locations.borrow_mut();
        block_locations.extend(response);
        Ok(())
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
        let mut client = self.client.clone();

        // If there is already a known block location before we do this
        let base_url = match self.block_locations.borrow().clone().get(&cid.to_string()) {
            Some(addresses) => Url::parse(&addresses[0])?,
            None => client.remote_data.clone(),
        };

        let mut stream = client
            .stream(PullBlock { cid: *cid }, &base_url)
            .await
            .map_err(|_| anyhow!("Failed to pull block"))?;
        println!("got a stream...");
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }
        println!("successfully retrieved block from api store...");
        Ok(Cow::Owned(data))
    }
}
