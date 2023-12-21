use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use wnfs::libipld::Cid;

use crate::api::requests::StreamableApiRequest;
use crate::prelude::api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct WriteBlock {
    pub cid: Cid,
    pub data: Vec<u8>,
    pub metadata_id: Uuid,
    pub completed: Option<()>,
}

#[derive(Debug, Deserialize)]
pub struct WriteBlockResponse;

impl ApiRequest for WriteBlock {
    type ErrorType = WriteBlockError;
    type ResponseType = WriteBlockResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        // TODO: Figure out how to get the block id
        let path = format!("/api/v1/blocks");
        let full_url = base_url.join(&path).unwrap();
        println!("full_url: {}", full_url);
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
        api::{
            error::ApiError, models::metadata::test::setup_and_push_metadata,
            requests::staging::upload::content::UploadContent,
        },
        blockstore::BanyanApiBlockStore,
        prelude::{
            api::{client::Client, requests::staging::blocks::WriteBlock},
            blockstore::BanyanBlockStore,
        },
    };
    use std::collections::BTreeSet;
    use url::Url;
    use wnfs::libipld::Cid;

    #[tokio::test]

    async fn write_cids() -> Result<(), ApiError> {
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

        setup.client.remote_core = Url::parse(&setup.storage_ticket.host).unwrap();

        for (i, cid) in cids.iter().enumerate() {
            let data = BanyanBlockStore::get_block(&setup.content_store, &cid)
                .await
                .unwrap()
                .to_vec();

            let completed = if i == cids.len() - 1 { Some(()) } else { None };

            let write_request = WriteBlock {
                cid: cid.to_owned(),
                data,
                metadata_id: setup.metadata.id,
                completed,
            };

            setup.client.call_no_content(write_request).await?;
        }

        Ok(())
    }
}
