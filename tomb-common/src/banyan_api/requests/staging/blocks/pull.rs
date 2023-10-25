use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use wnfs::libipld::Cid;

use crate::banyan_api::requests::StreamableApiRequest;

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

        println!("full_url of block request: {:?}", full_url.to_string());

        #[cfg(target_arch = "wasm32")]
        gloo::console::log!("full_url of block request: {:?}", full_url.to_string());

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
