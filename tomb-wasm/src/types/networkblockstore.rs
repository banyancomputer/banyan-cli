// use serde::{Serialize, Deserialize};
// use web_sys::{FormData, HtmlFormElement};
// use wnfs::{common::BlockStore, libipld::{IpldCodec, Cid}};
// use std::{net::Ipv4Addr, borrow::Cow};
// use async_trait::async_trait;
// use anyhow::Result;


// /// A network-based BlockStore designed to interface with a Kubo node or an API which mirrors it
// #[derive(Debug, Serialize, Deserialize)]
// pub struct NetworkBlockStore {
//     /// The address which we are connecting to
//     pub addr: String,
// }

// // -------------------------------------------------------------------------------------------------
// // Implementations
// // -------------------------------------------------------------------------------------------------

// impl NetworkBlockStore {
//     /// Initializes the NetworkBlockStore
//     pub fn new(ip: Ipv4Addr, port: u16) -> Self {
//         // Create/return the new instance of self
//         Self {
//             addr: format!("{}:{}", ip, port),
//         }
//     }
// }

// #[async_trait(?Send)]
// impl BlockStore for NetworkBlockStore {
//     /// Stores an array of bytes in the block store.
//     async fn put_block(&self, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid> {
//         // Try to build the CID from the bytes and codec
//         let cid = self.create_cid(&bytes, codec)?;

//         // Construct the appropriate URI for a block request
//         let url: String = format!("http://{}/api/v0/block/put/{}", self.addr, cid);
        
//         // let value = wasm_bindgen::JsValue::from(bytes);

//         // let formdata = FormData::from(bytes);
//         // let data = FormData::new();

//         // If we didn't receive an error response, we're Ok!
//         Ok(cid)
//     }

//     /// Retrieves an array of bytes from the block store with given CID.
//     async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>> {
//         // Return Ok status with the bytes
//         return Ok(Cow::Owned(Vec::new()));
//     }
// }