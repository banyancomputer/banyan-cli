use wnfs::libipld::Cid;

// use super::networkblockstore::NetworkBlockStore;


#[wasm_bindgen]
async fn send_string(value: String) {
    let store = NetworkBlockStore::new();

}

#[wasm_bindgen]
async fn get_string(cid: Cid) {

}