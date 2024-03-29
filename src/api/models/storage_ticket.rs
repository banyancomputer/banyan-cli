use crate::api::{
    client::Client, error::ApiError, requests::staging::client_grant::create::CreateGrant,
};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use tomb_crypt::prelude::{PrivateKey, PublicKey};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// StorageTicket is a ticket that can be used authenticate requests to stage data to a storage host
pub struct StorageTicket {
    /// The host to stage data to
    pub host: String,
    /// The authorization token to use when staging data. Generated by the core service
    pub authorization: String,
}

impl Display for StorageTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "\n{}\nhost:\t{}\nauthorization:\t{}",
            "| STORAGE TICKET INFO |".yellow(),
            self.host,
            self.authorization
        ))
    }
}

impl StorageTicket {
    /// Create a new grant for a client to stage data to a storage host
    /// Allows us to upload data to a storage host using our signing key
    pub async fn create_grant(&self, client: &mut Client) -> Result<(), ApiError> {
        let signing_key = client
            .signing_key
            .as_ref()
            .expect("Client signing key not set");
        let public_key_bytes = signing_key
            .public_key()
            .expect("Failed to get public key")
            .export()
            .await
            .expect("Failed to export public key");
        let public_key =
            String::from_utf8(public_key_bytes).expect("Failed to convert public key to string");
        client
            .call_no_content(CreateGrant {
                host_url: self.host.clone(),
                bearer_token: self.authorization.clone(),
                public_key,
            })
            .await
    }
}
