use anyhow::Result;
use async_trait::async_trait;
use reqwest::{
    multipart::{Form, Part},
    Client, Response,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{borrow::Cow, net::Ipv4Addr, str::from_utf8};
use wnfs::{
    common::BlockStore,
    libipld::{Cid, IpldCodec},
};

/// A network-based BlockStore designed to interface with a Kubo node or an API which mirrors it
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct NetworkBlockStore {
    /// The address which we are connecting to
    pub addr: String,
}

// -------------------------------------------------------------------------------------------------
// Implementations
// -------------------------------------------------------------------------------------------------

impl NetworkBlockStore {
    /// Initializes the NetworkBlockStore
    pub fn new(ip: Ipv4Addr, port: u16) -> Self {
        // Create/return the new instance of self
        Self {
            addr: format!("{}:{}", ip, port),
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
        let url: String = format!("http://{}/api/v0/block/put/{}", self.addr, cid);

        // Construct the Form data that will be sending content bytes over the network
        let form = Form::new().part("data", Part::bytes(bytes));

        // This command is modeled after the following curl command:
        // curl -X POST -F file=@myfile "http://127.0.0.1:5001/api/v0/block/put?cid-codec=raw&mhtype=sha2-256&mhlen=-1&pin=false&allow-big-block=false&format=<value>"
        let response: Response = Client::new()
            .post(url)
            .multipart(form)
            .send()
            .await
            .expect("Failed to send put_block request.");

        // Grab the Bytes response
        let bytes: Vec<u8> = response.bytes().await?.to_vec();
        // Represent these bytes as a plaintext response
        let plain: &str = std::str::from_utf8(&bytes)?;
        // Represent this as data traversable by serde
        let root: Value = serde_json::from_str(plain)?;
        // Expect to find the Key and Size keys in this json
        root.get("Key")
            .and(root.get("Size"))
            .expect("Server responded with an error.");

        // If we didn't receive an error response, we're Ok!
        Ok(cid)
    }

    /// Retrieves an array of bytes from the block store with given CID.
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
        // Construct the appropriate URI for a block request
        let url: String = format!("http://{}/api/v0/block/get?arg={}", self.addr, cid);

        // This command is modeled after the following curl command:
        // curl -X POST "http://127.0.0.1:5001/api/v0/block/get?arg=<cid>"
        let response: Response = Client::new()
            .post(url)
            .send()
            .await
            .expect("Failed to send get_block request.");

        // Grab the Bytes response
        let bytes: Vec<u8> = response.bytes().await?.to_vec();

        // If this is not a large response, it might be an error
        if bytes.len() < 300 {
            // If the Type field of the utf8 json response given was that of an error
            if serde_json::from_str(from_utf8(&bytes).unwrap_or("null"))
                .unwrap_or(Value::Null)
                .get("Type")
                .unwrap_or(&Value::Null)
                .as_str()
                .unwrap_or("null")
                == "Error"
            {
                // Panic time!
                panic!("Server responded with an error.");
            }
        }

        // Return Ok status with the bytes
        return Ok(Cow::Owned(bytes));
    }
}
