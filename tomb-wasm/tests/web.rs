//! Test suite for the Web and headless browsers.

use std::convert::TryFrom;

use gloo::console::log;
use gloo::utils::window;
use js_sys::{Array, Reflect, Uint8Array};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use web_sys::{CryptoKey, CryptoKeyPair};

use tomb_common::banyan_api::client::Client;
use tomb_common::banyan_api::models::account::Account;

use tomb_wasm::types::WasmFsMetadataEntry;
use tomb_wasm::{TombResult, TombWasm, WasmBucket, WasmBucketKey};

wasm_bindgen_test_configure!(run_in_browser);

const FIVE_TIB: u64 = 5_497_558_138_880;

fn js_array(values: &[&str]) -> JsValue {
    let js_array: Array = values.iter().map(|s| JsValue::from_str(s)).collect();

    JsValue::from(js_array)
}

pub async fn authenticated_client() -> TombResult<TombWasm> {
    let mut client = Client::new("http://127.0.0.1:3001").expect("client creation failed");

    let (account, _signing_key) = Account::create_fake(&mut client)
        .await
        .expect("fake account creation failed");
    assert_eq!(account.id.to_string(), client.subject().unwrap());

    let who_am_i = Account::who_am_i(&mut client)
        .await
        .expect("who_am_i failed");
    assert_eq!(account.id, who_am_i.id);

    Ok(TombWasm::from(client))
}

pub async fn create_bucket(
    client: &mut TombWasm,
    key_pair: &CryptoKeyPair,
) -> TombResult<WasmBucket> {
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

#[wasm_bindgen_test]
async fn get_usage() -> TombResult<()> {
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
async fn mount() -> TombResult<()> {
    log!("tomb_wasm_test: create_bucket_mount()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;

    assert!(!mount.locked());
    Ok(())
}

#[wasm_bindgen_test]
async fn share_with() -> TombResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_share_with()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let wasm_bucket_key: WasmBucketKey = client.create_bucket_key(bucket.id().to_string()).await?;
    assert_eq!(wasm_bucket_key.bucket_id(), bucket.id().to_string());
    assert!(!wasm_bucket_key.approved());
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert!(!mount.locked());
    mount.share_with(wasm_bucket_key.id()).await?;
    Ok(())
}

#[wasm_bindgen_test]
async fn mkdir() -> TombResult<()> {
    let mut client = authenticated_client().await?;

    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;

    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;

    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert!(!mount.locked());

    let mkdir_path_array: Array = js_array(&["test-dir"]).into();
    mount.mkdir(mkdir_path_array).await?;

    let ls_path_array: Array = js_array(&[]).into();
    let ls = mount.ls(ls_path_array).await?;
    assert_eq!(ls.length(), 1);

    let ls_0 = ls.get(0);
    let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();

    assert_eq!(fs_entry.name(), "test-dir");
    assert_eq!(fs_entry.entry_type(), "dir");

    Ok(())
}

#[wasm_bindgen_test]
async fn mkdir_remount() -> TombResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;

    log!("tomb_wasm_test: create_bucket_mount_mkdir_remount_ls(): create_bucket()");
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair.clone())
        .await?;
    assert!(!mount.locked());

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
    assert!(!mount.locked());
    let ls: Array = mount.ls(ls_path_array).await?;
    assert_eq!(ls.length(), 1);
    let ls_0 = ls.get(0);
    let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
    assert_eq!(fs_entry.name(), "test-dir");
    assert_eq!(fs_entry.entry_type(), "dir");
    Ok(())
}

#[wasm_bindgen_test]
async fn add() -> TombResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_mkdir()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert!(!mount.locked());
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
async fn add_read() -> TombResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_mkdir()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert!(!mount.locked());
    let add_path_array: Array = js_array(&["zero.bin"]).into();
    let ls_path_array: Array = js_array(&[]).into();
    let zero_content_buffer = Uint8Array::new_with_length(1024 * 1024 * 10);
    let zero_content_array_buffer = zero_content_buffer.buffer();
    mount
        .add(add_path_array.clone(), zero_content_array_buffer.clone())
        .await?;
    let ls: Array = mount.ls(ls_path_array).await?;
    assert_eq!(ls.length(), 1);
    let ls_0 = ls.get(0);
    let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
    assert_eq!(fs_entry.name(), "zero.bin");
    assert_eq!(fs_entry.entry_type(), "file");
    let new_bytes = mount.read_bytes(add_path_array, None).await?.to_vec();
    // Assert successful reconstruction
    assert_eq!(new_bytes, zero_content_buffer.to_vec());

    Ok(())
}

#[wasm_bindgen_test]
async fn add_remount() -> TombResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_add_ls_remount_ls()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;

    log!("tomb_wasm_test: create_bucket_mount_add_ls_remount_ls(): create_bucket()");
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair.clone())
        .await?;
    assert!(!mount.locked());

    log!("tomb_wasm_test: create_bucket_mount_add_ls_remount_ls(): add() and ls()");
    let add_path_array: Array = js_array(&["zero.bin"]).into();
    let ls_path_array: Array = js_array(&[]).into();
    let zero_content_buffer = Uint8Array::new_with_length(10);
    let zero_content_array_buffer = zero_content_buffer.buffer();
    mount.add(add_path_array, zero_content_array_buffer).await?;
    mount.mkdir(js_array(&["cats"]).into()).await?;
    let ls: Array = mount.ls(ls_path_array.clone()).await?;
    assert_eq!(ls.length(), 2);

    log!("tomb_wasm_test: create_bucket_mount_add_ls_remount_ls(): remount() and ls()");
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert!(!mount.locked());
    let ls: Array = mount.ls(ls_path_array).await?;
    assert_eq!(ls.length(), 2);
    let ls_0 = ls.get(1);
    let fs_entry = WasmFsMetadataEntry::try_from(ls_0).unwrap();
    assert_eq!(fs_entry.name(), "zero.bin");
    assert_eq!(fs_entry.entry_type(), "file");
    Ok(())
}

#[wasm_bindgen_test]
async fn add_rm() -> TombResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_add_rm()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert!(!mount.locked());
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
async fn add_mv() -> TombResult<()> {
    log!("tomb_wasm_test: create_bucket_mount_add_mv()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let bucket = create_bucket(&mut client, &web_encryption_key_pair).await?;
    let mut mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert!(!mount.locked());
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
