use anyhow::Result;
use async_trait::async_trait;
use reqwest::{
    multipart::{Form, Part},
    Client, Response,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{borrow::Cow, str::from_utf8}; // , time::Duration};
use thiserror::Error;
use wnfs::libipld::{Cid, IpldCodec};
use crate::blockstore::BlockStore;

/// A network-based BlockStore designed to interface with a Kubo node or an API which mirrors it
#[derive(Debug, Serialize, Deserialize, PartialEq, Default, Clone)]
pub struct NetworkBlockStore {
    /// The address which we are connecting to
    pub addr: String,
}

/// Network File errors.
#[derive(Debug, Error)]
pub(crate) enum NetworkError {
    #[error("Bad response given: {0}")]
    BadRespose(String),
    #[error("Endpoint was invalid: {0}")]
    BadEndpoint(String),
    #[error("No response given")]
    NoResponse,
}

// -------------------------------------------------------------------------------------------------
// Implementations
// -------------------------------------------------------------------------------------------------

impl NetworkBlockStore {
    /// Initializes the NetworkBlockStore
    pub fn new(addr: &str) -> Result<Self> {
        // TODO(organizedgrime) - also add a case for https
        if !addr.starts_with("http://") {
            Err(NetworkError::BadEndpoint(addr.to_string()).into())
        } else {
            // Create/return the new instance of self
            Ok(Self {
                addr: addr.to_string(),
            })
        }
    }
}

#[async_trait(?Send)]
impl BlockStore for NetworkBlockStore {
    /// Stores an array of bytes in the block store.
    async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
        // Try to build the CID from the bytes and codec
        let cid = self.create_cid(&bytes, codec)?;

        // Construct the appropriate URI for a block request
        let url: String = format!("{}/api/v0/block/put/{}", self.addr, cid);

        // Construct the Form data that will be sending content bytes over the network
        let form = Form::new().part("data", Part::bytes(bytes));

        // This command is modeled after the following curl command:
        // curl -X POST -F file=@myfile "http://127.0.0.1:5001/api/v0/block/put?cid-codec=raw&mhtype=sha2-256&mhlen=-1&pin=false&allow-big-block=false&format=<value>"
        if let Ok(response) = Client::new().post(url).multipart(form).send().await {
            // Grab the Bytes response
            let bytes: Vec<u8> = response.bytes().await?.to_vec();
            // Represent these bytes as a plaintext response
            let plain: &str = std::str::from_utf8(&bytes)?;
            // Represent this as data traversable by serde
            let root: Value = serde_json::from_str(plain)?;
            // Expect to find the Key and Size keys in this json
            if root.get("Key").is_some() && root.get("Size").is_some() {
                // Ok!
                Ok(cid)
            } else {
                Err(NetworkError::BadRespose("Missing Key and/or Size".to_string()).into())
            }
        } else {
            Err(NetworkError::NoResponse.into())
        }
    }

    /// Retrieves an array of bytes from the block store with given CID.
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        // Construct the appropriate URI for a block request
        let url: String = format!("{}/api/v0/block/get?arg={}", self.addr, cid);

        // This command is modeled after the following curl command:
        // curl -X POST "http://127.0.0.1:5001/api/v0/block/get?arg=<cid>"
        let response: Response = Client::new()
            .post(url)
            // TODO: Figure out why this won't compile
            // .timeout(Duration::SECOND)
            .send()
            .await
            .expect("Failed to send get_block request.");

        // Grab the Bytes response
        let bytes: Vec<u8> = response.bytes().await?.to_vec();

        // If this is not a large response, it might be an error
        if bytes.len() < 300 &&
            // If the Type field of the utf8 json response given was that of an error
            serde_json::from_str(from_utf8(&bytes).unwrap_or("null"))
                .unwrap_or(Value::Null)
                .get("Type")
                .unwrap_or(&Value::Null)
                .as_str()
                .unwrap_or("null")
                == "Error"
        {
            Err(NetworkError::BadRespose("Error Detected in response".to_string()).into())
        } else {
            // Return Ok status with the bytes
            Ok(Cow::Owned(bytes))
        }
    }
}
