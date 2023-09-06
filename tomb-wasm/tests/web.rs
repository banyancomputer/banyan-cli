//! Test suite for the Web and headless browsers.

use gloo::{console::log, utils::window};
use js_sys::{Array, Reflect, Uint8Array};
use std::convert::TryFrom;
use tomb_wasm::types::{WasmBucket, WasmBucketKey};
use wasm_bindgen_test::*;
use web_sys::CryptoKey;

use tomb_wasm::types::WasmFsMetadataEntry;
use tomb_wasm::utils::*;
use tomb_wasm::TombWasm;
extern crate tomb_wasm;
extern crate wasm_bindgen_test;

wasm_bindgen_test_configure!(run_in_browser);

use tomb_common::banyan_api::client::Client;
use tomb_common::banyan_api::models::account::Account;
use web_sys::CryptoKeyPair;

const FIVE_TIB: u64 = 5_497_558_138_880;

pub async fn authenticated_client() -> JsResult<TombWasm> {
    let mut client = Client::new("http://127.0.0.1:3001").expect("client creation failed");
    let (account, _signing_key) = Account::create_fake(&mut client)
        .await
        .expect("fake account creation failed");
    assert_eq!(account.id.to_string(), client.subject().unwrap());
    let who_am_i = Account::who_am_i(&mut client)
        .await
        .expect("who_am_i failed");
    assert_eq!(account.id.to_string(), who_am_i.id.to_string());
    Ok(TombWasm::from(client))
}

pub async fn create_bucket(
    client: &mut TombWasm,
    key_pair: &CryptoKeyPair,
) -> JsResult<WasmBucket> {
    let web_public_encryption_key =
        CryptoKey::from(Reflect::get(key_pair, &"publicKey".into()).unwrap());
    // Generate a random name
    let bucket_name = random_string(10);
    // Note: this might lint as an error, but it's not
    let bucket = client
        .create_bucket(
            bucket_name.clone(),
            "warm".to_string(),
            "interactive".to_string(),
            web_public_encryption_key,
        )
        .await?;
    assert_eq!(bucket.name(), bucket_name);
    assert_eq!(bucket.storage_class(), "warm");
    assert_eq!(bucket.bucket_type(), "interactive");
    Ok(bucket)
}

#[wasm_bindgen_test]
async fn get_usage() -> JsResult<()> {
    log!("tomb_wasm_test: get_usage()");
    let _key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let mut client = authenticated_client().await?;
    let usage = client.get_usage().await?;
    assert_eq!(usage, 0);
    let usage_limit = client.get_usage_limit().await?;
    assert_eq!(usage_limit, FIVE_TIB);
    Ok(())
}

// TODO: probably for API tests

#[wasm_bindgen_test]
async fn mount() -> JsResult<()> {
    log!("tomb_wasm_test: create_bucket_mount()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert_eq!(mount.locked(), false);
    Ok(())
}

#[wasm_bindgen_test]
async fn share_with() -> JsResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_share_with()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let wasm_bucket_key: WasmBucketKey = client
        .create_bucket_key(bucket.id().to_string())
        .await?
        .into();
    assert_eq!(wasm_bucket_key.bucket_id(), bucket.id().to_string());
    assert_eq!(wasm_bucket_key.approved(), false);
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert_eq!(mount.locked(), false);
    mount.share_with(wasm_bucket_key.id()).await?;
    Ok(())
}

#[wasm_bindgen_test]
async fn mkdir() -> JsResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_mkdir()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert_eq!(mount.locked(), false);
    let mkdir_path_array: Array = js_array(&["test-dir"]).into();
    let ls_path_array: Array = js_array(&[]).into();
    mount.mkdir(mkdir_path_array).await?;
    let ls: Array = mount.ls(ls_path_array).await?;
    assert_eq!(ls.length(), 1);
    let ls_0 = ls.get(0);
    let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
    assert_eq!(fs_entry.name(), "test-dir");
    assert_eq!(fs_entry.entry_type(), "dir");
    Ok(())
}

#[wasm_bindgen_test]
async fn mkdir_remount() -> JsResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;

    log!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls(): create_bucket()");
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair.clone())
        .await?;
    assert_eq!(mount.locked(), false);

    log!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls(): mkdir() and ls()");
    let mkdir_path_array: Array = js_array(&["test-dir"]).into();
    let ls_path_array: Array = js_array(&[]).into();
    mount.mkdir(mkdir_path_array).await?;
    let ls: Array = mount.ls(ls_path_array.clone()).await?;
    assert_eq!(ls.length(), 1);

    log!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls(): remount() and ls()");
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert_eq!(mount.locked(), false);
    let ls: Array = mount.ls(ls_path_array).await?;
    assert_eq!(ls.length(), 1);
    let ls_0 = ls.get(0);
    let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
    assert_eq!(fs_entry.name(), "test-dir");
    assert_eq!(fs_entry.entry_type(), "dir");
    Ok(())
}

#[wasm_bindgen_test]
#[should_panic]
async fn add() -> JsResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_mkdir()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert_eq!(mount.locked(), false);
    let add_path_array: Array = js_array(&["zero.bin"]).into();
    let ls_path_array: Array = js_array(&[]).into();
    let zero_content_buffer = Uint8Array::new_with_length(10);
    let zero_content_array_buffer = zero_content_buffer.buffer();
    mount.add(add_path_array, zero_content_array_buffer).await?;
    let ls: Array = mount.ls(ls_path_array).await?;
    assert_eq!(ls.length(), 1);
    let ls_0 = ls.get(0);
    let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
    assert_eq!(fs_entry.name(), "zero.bin");
    assert_eq!(fs_entry.entry_type(), "file");
    Ok(())
}

#[wasm_bindgen_test]
#[should_panic]
async fn add_remount() -> JsResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_add_ls_remount_ls()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;

    log!("tomb_wasm_test: create_bucket_mount_add_ls_remount_ls(): create_bucket()");
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair.clone())
        .await?;
    assert_eq!(mount.locked(), false);

    log!("tomb_wasm_test: create_bucket_mount_add_ls_remount_ls(): add() and ls()");
    let add_path_array: Array = js_array(&["zero.bin"]).into();
    let ls_path_array: Array = js_array(&[]).into();
    let zero_content_buffer = Uint8Array::new_with_length(10);
    let zero_content_array_buffer = zero_content_buffer.buffer();
    mount.add(add_path_array, zero_content_array_buffer).await?;
    let ls: Array = mount.ls(ls_path_array.clone()).await?;
    assert_eq!(ls.length(), 1);

    log!("tomb_wasm_test: create_bucket_mount_add_ls_remount_ls(): remount() and ls()");
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert_eq!(mount.locked(), false);
    let ls: Array = mount.ls(ls_path_array).await?;
    assert_eq!(ls.length(), 1);
    let ls_0 = ls.get(0);
    let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
    assert_eq!(fs_entry.name(), "zero.bin");
    assert_eq!(fs_entry.entry_type(), "file");
    Ok(())
}

#[wasm_bindgen_test]
#[should_panic]
async fn add_rm() -> JsResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_add_rm()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert_eq!(mount.locked(), false);
    let add_path_array: Array = js_array(&["zero.bin"]).into();
    let ls_path_array: Array = js_array(&[]).into();
    let zero_content_buffer = Uint8Array::new_with_length(10);
    let zero_content_array_buffer = zero_content_buffer.buffer();
    mount.add(add_path_array, zero_content_array_buffer).await?;
    let ls: Array = mount.ls(ls_path_array.clone()).await?;
    assert_eq!(ls.length(), 1);
    let rm_path_array: Array = js_array(&["zero.bin"]).into();
    mount.rm(rm_path_array).await?;
    let ls: Array = mount.ls(ls_path_array).await?;
    assert_eq!(ls.length(), 0);
    Ok(())
}

#[wasm_bindgen_test]
#[should_panic]
async fn add_mv() -> JsResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_add_mv()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert_eq!(mount.locked(), false);
    let add_path_array: Array = js_array(&["zero.bin"]).into();
    let ls_path_array: Array = js_array(&[]).into();
    let zero_content_buffer = Uint8Array::new_with_length(10);
    let zero_content_array_buffer = zero_content_buffer.buffer();
    mount.add(add_path_array, zero_content_array_buffer).await?;
    let ls: Array = mount.ls(ls_path_array.clone()).await?;
    assert_eq!(ls.length(), 1);
    let mv_from_path_array: Array = js_array(&["zero.bin"]).into();
    let mv_to_path_array: Array = js_array(&["zero-renamed.bin"]).into();
    mount.mv(mv_from_path_array, mv_to_path_array).await?;
    let ls: Array = mount.ls(ls_path_array).await?;
    assert_eq!(ls.length(), 1);
    let ls_0 = ls.get(0);
    let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
    assert_eq!(fs_entry.name(), "zero-renamed.bin");
    assert_eq!(fs_entry.entry_type(), "file");
    Ok(())
}

fn random_string(length: usize) -> String {
    use rand::{thread_rng, Rng};
    let mut rng = thread_rng();
    let bytes = (0..length)
        .map(|_| rng.sample(rand::distributions::Alphanumeric))
        .collect();
    String::from_utf8(bytes).unwrap()
}

async fn web_ec_key_pair(key_type: &str, uses: &[&str]) -> CryptoKeyPair {
    let subtle = window().crypto().unwrap().subtle();
    let params = web_sys::EcKeyGenParams::new(key_type, "P-384");
    let usages = js_array(uses);
    let promise = subtle
        .generate_key_with_object(&params, true, &usages)
        .unwrap();
    let key_pair = wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
    CryptoKeyPair::from(key_pair)
}
