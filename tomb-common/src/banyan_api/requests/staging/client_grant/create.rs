use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};

use crate::banyan_api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct CreateGrant {
    pub host_url: String,
    pub bearer_token: String,
    pub public_key: String,
}

#[derive(Debug, Serialize)]
struct CreateGrantData {
    pub public_key: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateGrantResponse {}

impl ApiRequest for CreateGrant {
    type ResponseType = CreateGrantResponse;
    type ErrorType = CreateGrantError;

    fn build_request(self, _base_url: &Url, client: &Client) -> RequestBuilder {
        // Ignore the client url, and use our own bearer token
        let base_url = Url::parse(&self.host_url).unwrap();
        let full_url = base_url.join("/api/v1/client_grant").unwrap();
        client
            .post(full_url)
            .json(&CreateGrantData {
                public_key: self.public_key,
            })
            .bearer_auth(self.bearer_token)
    }

    fn requires_authentication(&self) -> bool {
        false
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct CreateGrantError {
    #[serde(rename = "error")]
    kind: CreateGrantErrorKind,
}

impl Display for CreateGrantError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use CreateGrantErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred creating the bucket",
        };

        f.write_str(msg)
    }
}

impl Error for CreateGrantError {}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
enum CreateGrantErrorKind {
    Unknown,
}
