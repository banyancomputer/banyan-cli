use crate::prelude::api::requests::ApiRequest;
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;
use wnfs::libipld::Cid;

#[derive(Debug, Serialize)]
pub struct WriteBlock {
    pub cid: Cid,
    pub data: Vec<u8>,
    pub metadata_id: Uuid,
    pub completed: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct WriteBlockResponse;

impl ApiRequest for WriteBlock {
    type ErrorType = WriteBlockError;
    type ResponseType = WriteBlockResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v1/blocks").unwrap();
        client.post(full_url).json(&self)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct WriteBlockError {
    #[serde(rename = "msg")]
    message: String,
}

impl Display for WriteBlockError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.message.as_ref())
    }
}

impl Error for WriteBlockError {}

#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod test {
    use crate::{
        api::error::ApiError,
        prelude::{
            api::{
                models::metadata::test::setup_and_push_metadata,
                requests::staging::blocks::WriteBlock,
            },
            blockstore::{BanyanApiBlockStore, BanyanBlockStore},
        },
    };
    use std::{collections::BTreeSet, thread};
    use url::Url;
    use wnfs::libipld::Cid;

    #[tokio::test]

    async fn write_cids_read_cids() -> Result<(), ApiError> {
        let mut setup = setup_and_push_metadata("download_content").await?;
        // Create a grant and upload content
        setup
            .storage_ticket
            .clone()
            .create_grant(&mut setup.client)
            .await?;

        // Sleep to allow block locations to be updated
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let mut cids = <BTreeSet<Cid>>::new();
        for bucket in setup.content_store.car.car.index.borrow().clone().buckets {
            cids.extend(bucket.map.into_keys().collect::<BTreeSet<Cid>>());
        }

        let core_client = setup.client.clone();
        setup.client.remote_core = Url::parse(&setup.storage_ticket.host).unwrap();

        for (i, cid) in cids.iter().enumerate() {
            let data = BanyanBlockStore::get_block(&setup.content_store, cid)
                .await
                .unwrap()
                .to_vec();

            let completed = if i == cids.len() - 1 {
                Some(true)
            } else {
                None
            };

            println!("completed: {:?}", completed);

            let write_request = WriteBlock {
                cid: cid.to_owned(),
                data,
                metadata_id: setup.metadata.id,
                completed,
            };

            setup.client.call_no_content(write_request).await?;
        }

        thread::sleep(std::time::Duration::new(3, 0));

        let api_store = BanyanApiBlockStore::from(core_client);
        api_store.find_cids(cids.clone()).await?;

        for cid in &cids {
            let block = BanyanBlockStore::get_block(&api_store, cid).await?.to_vec();
            assert_eq!(setup.content_store.get_block(cid).await?.to_vec(), block);
        }

        Ok(())
    }
}
