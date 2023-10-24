use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::Deserialize;

use crate::banyan_api::requests::ApiRequest;

pub type LocationRequest = Vec<cid::Cid>;

pub type LocationResponse = std::collections::HashMap<String, Vec<cid::Cid>>;

impl ApiRequest for LocationRequest {
    type ResponseType = LocationResponse;
    type ErrorType = LocationRequestError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v1/blocks/locate").unwrap();
        client.post(full_url).json(&self)
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
