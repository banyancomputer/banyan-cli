use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::api::requests::ApiRequest;
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use wnfs::libipld::Cid;

#[derive(Debug, Serialize, Deserialize)]
pub struct LocationRequest {
    pub cids: BTreeSet<Cid>,
}

pub type LocationResponse = std::collections::HashMap<String, Vec<String>>;

impl ApiRequest for LocationRequest {
    type ResponseType = LocationResponse;
    type ErrorType = LocationRequestError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v1/blocks/locate").unwrap();
        client.post(full_url).json(
            &self
                .cids
                .iter()
                .map(|cid| cid.to_string())
                .collect::<Vec<String>>(),
        )
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct LocationRequestError {
    msg: String,
}

impl Display for LocationRequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl Error for LocationRequestError {}

/*
#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod test {
    use std::collections::BTreeSet;

    use crate::{
        api::{
            error::ApiError,
            models::{
                account::test::authenticated_client, metadata::test::setup_and_push_metadata,
            },
            requests::{
                core::blocks::locate::LocationRequest, staging::upload::content::UploadContent,
            },
        },
        blockstore::{BanyanApiBlockStore, DoubleSplitStore},
    };
    use serial_test::serial;
    use wnfs::libipld::Cid;

    #[tokio::test]
    #[serial]
    async fn get_locations() -> Result<(), ApiError> {
        let mut setup = setup_and_push_metadata("get_locations").await?;
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

        let mut blockstore_client = setup.client.clone();
        blockstore_client
            .with_remote(&setup.storage_ticket.host)
            .expect("Failed to create blockstore client");
        let api_blockstore = BanyanApiBlockStore::from(blockstore_client);
        let node = setup
            .fs
            .get_node(&["cat.txt".to_string()], &setup.metadata_store)
            .await?
            .unwrap();
        let file = node.as_file()?;
        let split_store = DoubleSplitStore::new(&api_blockstore, &setup.metadata_store);
        let cids = file.get_cids(&setup.fs.forest, &split_store).await?;
        let cids_request = LocationRequest { cids: cids.clone() };
        let locations = setup
            .client
            .call(cids_request)
            .await
            .expect("Failed to get locations");

        let stored_blocks = locations
            .get(&setup.storage_ticket.host)
            .expect("no blocks at storage host");
        for cid in cids {
            assert!(stored_blocks.contains(&cid.to_string()));
        }
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn get_bad_location() -> Result<(), ApiError> {
        let mut client = authenticated_client().await;
        let mut cids = BTreeSet::new();
        cids.insert(Cid::default());
        let location_request = LocationRequest { cids: cids.clone() };
        let locations = client
            .call(location_request)
            .await
            .expect("Failed to get locations");
        let target_cids = locations.get("NA").expect("Failed to get cids");
        for cid in cids.clone() {
            assert!(target_cids.contains(&cid.to_string()));
        }
        Ok(())
    }
}
*/
