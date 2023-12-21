use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use wnfs::libipld::Cid;

use crate::api::requests::StreamableApiRequest;

#[derive(Debug, Serialize)]
pub struct ReadBlock {
    pub cid: Cid,
}

#[derive(Debug, Deserialize)]
pub struct ReadBlockResponse(pub(crate) Vec<u8>);

impl StreamableApiRequest for ReadBlock {
    type ErrorType = ReadBlockError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        // TODO: Figure out how to get the block id
        let block_id = self.cid.to_string();
        let path = format!("/api/v1/blocks/{}", block_id);
        let full_url = base_url.join(&path).unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ReadBlockError {
    #[serde(rename = "msg")]
    message: String,
}

impl Display for ReadBlockError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.message.as_ref())
    }
}

impl Error for ReadBlockError {}

#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod test {
    use crate::{
        api::{
            error::ApiError, models::metadata::test::setup_and_push_metadata,
            requests::staging::upload::content::UploadContent,
        },
        blockstore::BanyanApiBlockStore,
        prelude::blockstore::BanyanBlockStore,
    };
    use std::collections::BTreeSet;
    use wnfs::libipld::Cid;

    #[tokio::test]

    async fn download_content() -> Result<(), ApiError> {
        let mut setup = setup_and_push_metadata("download_content").await?;
        // Create a grant and upload content
        setup
            .storage_ticket
            .clone()
            .create_grant(&mut setup.client)
            .await?;
        setup
            .content_store
            .upload(
                setup.storage_ticket.host.clone(),
                setup.metadata.id,
                &mut setup.client,
            )
            .await?;

        // Sleep to allow block locations to be updated
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let mut cids = <BTreeSet<Cid>>::new();
        for bucket in setup.content_store.car.car.index.borrow().clone().buckets {
            cids.extend(bucket.map.into_keys().collect::<BTreeSet<Cid>>());
        }

        let api_store = BanyanApiBlockStore::from(setup.client);
        api_store.find_cids(cids.clone()).await?;

        for cid in &cids {
            let block = BanyanBlockStore::get_block(&api_store, cid).await?.to_vec();
            assert_eq!(setup.content_store.get_block(cid).await?.to_vec(), block);
        }

        Ok(())
    }
}
