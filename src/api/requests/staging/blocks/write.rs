use crate::prelude::api::requests::ApiRequest;
use reqwest::multipart::{Form, Part};
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;
use wnfs::libipld::Cid;

#[derive(Debug, Serialize)]
pub struct NewUpload {
    pub metadata_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct NewUploadResponse {
    upload_id: String,
}

impl ApiRequest for NewUpload {
    type ErrorType = BlockUploadError;
    type ResponseType = NewUploadResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v1/upload/new").unwrap();
        client.post(full_url).json(&self)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}
#[derive(Debug, Serialize)]
pub struct BlockUpload {
    pub cid: Cid,
    #[serde(flatten)]
    pub details: BlockUploadDetails,
    #[serde(skip)]
    pub data: Vec<u8>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum BlockUploadDetails {
    Ongoing { completed: bool, upload_id: String },
    OneOff,
}

#[derive(Debug, Deserialize)]
pub struct BlockUploadResonse;

impl ApiRequest for BlockUpload {
    type ErrorType = BlockUploadError;
    type ResponseType = BlockUploadResonse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v1/upload/block").unwrap();
        println!("full_url: {full_url}");
        let content_len = self.data.len() + 1000;
        // Attach the form data to the request as json
        let multipart_json_data = serde_json::to_string(&self).unwrap();
        println!("multipart_json_data: {multipart_json_data}");

        // Create a part for the json data
        let multipart_json = Part::bytes(multipart_json_data.as_bytes().to_vec())
            .mime_str("application/json")
            .unwrap();
        // Create a Part for the block data
        let multipart_block = Part::stream(self.data)
            .mime_str("application/octet-stream")
            .unwrap();

        // Combine the two parts into a multipart form
        let multipart_form = Form::new()
            .part("request-data", multipart_json)
            .part("block", multipart_block);

        client
            .post(full_url)
            .multipart(multipart_form)
            .header(reqwest::header::CONTENT_LENGTH, content_len)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct BlockUploadError {
    #[serde(rename = "msg")]
    message: String,
}

impl Display for BlockUploadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.message.as_ref())
    }
}

impl Error for BlockUploadError {}

#[cfg(test)]
#[cfg(feature = "integration-tests")]
mod test {
    use crate::{
        api::error::ApiError,
        prelude::{
            api::{
                models::metadata::test::setup_and_push_metadata,
                requests::staging::blocks::{
                    BlockUpload, BlockUploadDetails, NewUpload, NewUploadResponse,
                },
            },
            blockstore::{BanyanApiBlockStore, BanyanBlockStore},
        },
    };
    use std::{collections::BTreeSet, thread};
    use url::Url;
    use wnfs::libipld::Cid;

    #[tokio::test]
    async fn write_cids_read_cids() -> Result<(), ApiError> {
        let mut setup = setup_and_push_metadata("write_cids_read_cids").await?;
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

        let new_upload_response: NewUploadResponse = setup
            .client
            .call(NewUpload {
                metadata_id: setup.metadata.id,
            })
            .await?;

        println!("new_upload_response: {:?}", new_upload_response);

        for (i, cid) in cids.iter().enumerate() {
            let data = BanyanBlockStore::get_block(&setup.content_store, cid)
                .await
                .unwrap()
                .to_vec();

            let completed = i == cids.len() - 1;

            println!("completed: {:?}", completed);

            let upload_request = BlockUpload {
                cid: cid.to_owned(),
                data,
                details: BlockUploadDetails::Ongoing {
                    completed,
                    upload_id: new_upload_response.upload_id.clone(),
                },
            };

            println!("request: {:?}", upload_request);
            setup.client.multipart_no_content(upload_request).await?;
        }

        thread::sleep(std::time::Duration::new(3, 0));
        /*

        let api_store = BanyanApiBlockStore::from(core_client);
        api_store.find_cids(cids.clone()).await?;

        for cid in &cids {
            let block = BanyanBlockStore::get_block(&api_store, cid).await?.to_vec();
            assert_eq!(setup.content_store.get_block(cid).await?.to_vec(), block);
        }

            */
        Ok(())
    }
}
