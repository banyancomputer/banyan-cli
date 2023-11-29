use crate::api::{
    client::Client,
    error::ApiError,
    requests::{core::blocks::locate::LocationRequest, staging::pull_blocks::PullBlock},
};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Url;
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::{BTreeSet, HashMap},
};
use wnfs::libipld::{Cid, IpldCodec};

use super::{BanyanBlockStore, BlockStoreError};

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
    pub async fn find_cids(&self, cids: BTreeSet<Cid>) -> Result<(), ApiError> {
        let request = LocationRequest { cids };
        let mut client = self.client.clone();
        let response = client.call(request).await?;
        let mut block_locations = self.block_locations.borrow_mut();
        block_locations.extend(response);
        Ok(())
    }
}

#[async_trait(?Send)]
impl BanyanBlockStore for BanyanApiBlockStore {
    /// Stores an array of bytes in the block store.
    async fn put_block(&self, _bytes: Vec<u8>, _codec: IpldCodec) -> Result<Cid, BlockStoreError> {
        Err(BlockStoreError::wnfs(Box::from(
            "Cannot put block in API store",
        )))
    }

    /// Retrieves an array of bytes from the block store with given CID.
    #[allow(clippy::await_holding_refcell_ref)]
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, BlockStoreError> {
        let mut client = self.client.clone();

        // Pull the first url that has the block from the map of url: [block_id]
        let mut maybe_url = None;
        for (url, cids) in self.block_locations.borrow().iter() {
            if cids.contains(&cid.to_string()) {
                let base_url =
                    Url::parse(url).map_err(|_| BlockStoreError::wnfs(Box::from("url parse")))?;
                maybe_url = Some(base_url);
                break;
            }
        }

        let base_url = match maybe_url {
            Some(url) => url,
            None => client.remote_data.clone(),
        };

        let mut stream = client
            .stream(PullBlock { cid: *cid }, &base_url)
            .await
            .map_err(|err| BlockStoreError::wnfs(Box::from(err)))?;
        let mut data = Vec::new();
        while let Some(chunk) = stream.next().await {
            data.extend_from_slice(&chunk.unwrap());
        }
        Ok(Cow::Owned(data))
    }
}
