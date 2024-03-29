use std::error::Error;
use std::fmt::{self, Display, Formatter};

use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::requests::ApiRequest;

#[derive(Debug, Serialize)]
pub struct AuthorizationGrants {
    pub bucket_id: Uuid,
}

#[derive(Debug, Serialize)]
struct AuthorizationGrantsData {
    pub bucket_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct AuthorizationGrantsResponse {
    pub authorization_token: String,
}

impl ApiRequest for AuthorizationGrants {
    type ResponseType = AuthorizationGrantsResponse;
    type ErrorType = AuthorizationGrantsError;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        // Ignore the client url, and use our own bearer token
        let full_url = base_url
            .join(&format!(
                "/api/v1/buckets/{}/authorization_grants",
                self.bucket_id
            ))
            .unwrap();
        client.get(full_url).json(&AuthorizationGrantsData {
            bucket_id: self.bucket_id,
        })
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct AuthorizationGrantsError {
    #[serde(rename = "msg")]
    message: String,
}

impl Display for AuthorizationGrantsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.message.as_str())
    }
}

impl Error for AuthorizationGrantsError {}
