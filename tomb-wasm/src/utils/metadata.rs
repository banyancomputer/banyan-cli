// use anyhow::Result;
// use crate::types::pipeline::Manifest;
use tomb_common::types::blockstore::networkblockstore::NetworkBlockStore;
use wasm_bindgen::{prelude::wasm_bindgen, JsError};
use wnfs::{
    common::BlockStore,
    libipld::{Cid, IpldCodec},
};

#[wasm_bindgen]
pub async fn save_metadata(value: String) -> Result<String, JsError> {
    let store: &NetworkBlockStore = &NetworkBlockStore::new("http://127.0.0.1:5001").unwrap();
    let cid = store
        .put_block(value.as_bytes().to_vec(), IpldCodec::Raw)
        .await
        .unwrap();
    Ok(cid.to_string())
}

#[wasm_bindgen]
pub async fn load_metadata(cid: String) -> Result<String, JsError> {
    let store: &NetworkBlockStore = &NetworkBlockStore::new("http://127.0.0.1:5001").unwrap();
    let bytes = store.get_block(&Cid::try_from(cid)?).await.unwrap();
    let value: String = std::str::from_utf8(&bytes)?.to_string();
    Ok(value)
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn simple() {
        assert!(true);
    }
}
