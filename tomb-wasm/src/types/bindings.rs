use tomb_common::types::blockstore::networkblockstore::NetworkBlockStore;
use wasm_bindgen::{prelude::wasm_bindgen, JsError};
use wnfs::{
    common::BlockStore,
    libipld::{Cid, IpldCodec},
};

#[wasm_bindgen]
pub async fn send_string(value: String) -> Result<String, JsError> {
    let store: &NetworkBlockStore = &NetworkBlockStore::new("http://127.0.0.1", 5001);
    let cid = store
        .put_block(value.as_bytes().to_vec(), IpldCodec::Raw)
        .await
        .unwrap();
    Ok(cid.to_string())
}

#[wasm_bindgen]
pub async fn retrieve_string(cid: String) -> Result<String, JsError> {
    let store: &NetworkBlockStore = &NetworkBlockStore::new("http://127.0.0.1", 5001);
    let bytes = store.get_block(&Cid::try_from(cid)?).await.unwrap();
    let value: String = std::str::from_utf8(&bytes)?.to_string();
    Ok(value)
}

#[wasm_bindgen]
pub async fn pack() {
    
}
