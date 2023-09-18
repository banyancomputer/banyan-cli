use std::{convert::TryFrom, fs};
use gloo::console::log;
use gloo::utils::window;
use js_sys::{Array, Reflect, Uint8Array};
use rand::thread_rng;
use tomb_crypt::prelude::{EcEncryptionKey, PrivateKey};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use web_sys::{CryptoKey, CryptoKeyPair};

use tomb_common::banyan_api::client::Client;
use tomb_common::banyan_api::models::account::Account;

use tomb_wasm::types::WasmFsMetadataEntry;
use tomb_common::metadata::FsMetadata;
use tomb_common::blockstore::carv2_memory::CarV2MemoryBlockStore;
use tomb_wasm::{TombResult, TombWasm, WasmBucket, WasmBucketKey};
use wnfs::private::{PrivateDirectory, PrivateForest};
use wnfs::namefilter::Namefilter;
use std::rc::Rc;
use chrono::Utc;

wasm_bindgen_test_configure!(run_in_browser);

const FIVE_TIB: u64 = 5_497_558_138_880;

fn js_array(values: &[&str]) -> JsValue {
    let js_array: Array = values.iter().map(|s| JsValue::from_str(s)).collect();

    JsValue::from(js_array)
}

/*
#[wasm_bindgen_test]
async fn fs_test() -> TombResult<()> {
    log!("tomb_wasm_test: fs_test()");
    let key = EcEncryptionKey::generate().await?;
    let mut fs_metadata = FsMetadata::init(&key)
            .await
            .expect("could not init fs metadata");
    
    // Create a new blockstores
    let metadata_store = &mut CarV2MemoryBlockStore::new().expect("unable to create new blockstore");
    let content_store = &mut CarV2MemoryBlockStore::new().expect("unable to create new blockstore");
    // List files
    let entries = fs_metadata.ls(vec![], metadata_store).await.expect("ls1");
    // Assert none present
    assert!(entries.is_empty());
    // Add a new file
    fs_metadata.add(vec!["cat.txt".to_string()], "hello kitty".as_bytes().to_vec(), metadata_store, content_store).await.expect("add");
    // List files again
    let entries = fs_metadata.ls(vec![], metadata_store).await.expect("ls2");
    assert_eq!(entries.len(), 1);
    let file = fs_metadata.get_node(vec![entries[0].name.clone()], metadata_store).await.expect("get_node").expect("none found");
    let content = file.as_file().expect("not a file").get_content(&fs_metadata.content_forest, content_store).await.expect("get content");
    let string = String::from_utf8_lossy(&content).to_string();
    log!(format!("here is the entry's reconstructed data: {:?}", string));

    Ok(())
}
 */

#[wasm_bindgen_test]
async fn bs_test() -> TombResult<()> {
    log!("tomb_wasm_test: bs_test()");

    let time = Utc::now();
    let rng = &mut thread_rng();

    // Create a new PrivateForest for our metadata blocks
    let metadata_forest = &mut Rc::new(PrivateForest::new());
    // Create a new PrivateForest for our content holding blocks
    let content_forest = &mut Rc::new(PrivateForest::new());
    // Create a new blockstores
    let metadata_store = &mut CarV2MemoryBlockStore::new().expect("unable to create new blockstore");
    let content_store = &mut CarV2MemoryBlockStore::new().expect("unable to create new blockstore");

    // Create a new PrivateDirectory for the root of the Fs
    let mut root_dir = Rc::new(PrivateDirectory::new(
        Namefilter::default(),
        time,
        rng
    ));

    let file = root_dir.open_file_mut(
        &vec!["cat.txt".to_string()],
        true,
        time,
        metadata_forest,
        metadata_store,
        rng
    ).await.expect("open_file_mut");

    file.set_content(
        time,
        "hello kitty!".as_bytes(),
        content_forest,
        content_store,
        rng
    ).await.expect("set_content");

    let node = root_dir.get_node(&vec!["cat.txt".to_string()], true, metadata_forest, metadata_store).await.expect("get node").expect("no node");
    let content = node.as_file().expect("not a file").get_content(content_forest, content_store).await.expect("get content");
    let content_string = String::from_utf8_lossy(&content).to_string();
    assert_eq!(content_string, "hello kitty!".to_string());

    // List files again
    Ok(())
}