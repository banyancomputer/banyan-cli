use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::banyan_api::requests::ApiRequest;
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
