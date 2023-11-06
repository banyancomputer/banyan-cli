use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use wnfs::libipld::Cid;

use crate::banyan_common::banyan_api::requests::StreamableApiRequest;

#[derive(Debug, Serialize)]
pub struct PullBlock {
    pub cid: Cid,
}

#[derive(Debug, Deserialize)]
pub struct PullBlockResponse(pub(crate) Vec<u8>);

impl StreamableApiRequest for PullBlock {
    type ErrorType = PullBlockError;

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
pub struct PullBlockError {
    #[serde(rename = "msg")]
    message: String,
}

impl Display for PullBlockError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.message.as_ref())
    }
}

impl Error for PullBlockError {}

#[cfg(test)]
#[cfg(feature = "fake")]
mod test {
    use std::collections::BTreeSet;

    use crate::banyan_common::banyan_api::{
        blockstore::BanyanApiBlockStore, error::ClientError,
        models::metadata::test::setup_and_push_metadata,
        requests::staging::upload::content::UploadContent,
    };
    use cid::Cid;
    use serial_test::serial;
    use wnfs::common::BlockStore;

    #[tokio::test]
    #[serial]
    async fn download_content() -> Result<(), ClientError> {
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

        let mut cids = <BTreeSet<Cid>>::new();
        for bucket in setup.content_store.car.car.index.borrow().clone().buckets {
            cids.extend(bucket.map.into_keys().collect::<BTreeSet<Cid>>());
        }

        let banyan_api_store = BanyanApiBlockStore::from(setup.client);
        banyan_api_store.find_cids(cids.clone()).await?;

        for cid in &cids {
            assert!(banyan_api_store.get_block(cid).await.is_ok());
        }

        Ok(())
    }
}
