/*
//! Test suite for the Web and headless browsers.

use gloo::{console::log, utils::window};
use js_sys::{Array, Reflect};
use wasm_bindgen_test::*;
use web_sys::CryptoKey;

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
    let mut client = Client::new("http://localhost:3001").expect("client creation failed");
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

#[wasm_bindgen_test]
async fn authenticate_client() -> JsResult<()> {
    let _client = authenticated_client().await?;
    Ok(())
}

#[wasm_bindgen_test]
async fn create_bucket() -> JsResult<()> {
    log!("tomb_wasm_test: create_bucket()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let web_public_encryption_key =
        CryptoKey::from(Reflect::get(&web_encryption_key_pair, &"publicKey".into()).unwrap());
    // Note: this might lint as an error, but it's not
    let bucket = client
        .create_bucket(
            "test-bucket".to_string(),
            "warm".to_string(),
            "interactive".to_string(),
            web_public_encryption_key,
        )
        .await?;
    assert_eq!(bucket.name(), "test-bucket");
    assert_eq!(bucket.storage_class(), "warm");
    assert_eq!(bucket.bucket_type(), "interactive");
    Ok(())
}

#[wasm_bindgen_test]
#[should_panic]
async fn create_mount_bucket() -> JsResult<()> {
    log!("tomb_wasm_test: create_mount_bucket()");
    let mut client = authenticated_client().await?;
    let web_encryption_key_pair = web_ec_key_pair("ECDH", &["deriveBits"]).await;
    let web_public_encryption_key =
        CryptoKey::from(Reflect::get(&web_encryption_key_pair, &"publicKey".into()).unwrap());
    // Note: this might lint as an error, but it's not
    let bucket = client
        .create_bucket(
            "test-bucket".to_string(),
            "warm".to_string(),
            "mount".to_string(),
            web_public_encryption_key,
        )
        .await?;
    assert_eq!(bucket.name(), "test-bucket");
    assert_eq!(bucket.storage_class(), "warm");
    assert_eq!(bucket.bucket_type(), "interactive");

    let mount = client
        .mount(bucket.id().to_string(), web_encryption_key_pair)
        .await?;
    assert_eq!(mount.is_locked(), false);
    let ls: Array = mount.ls("/".to_string()).await?;
    println!("ls: {:?}", ls);
    assert_eq!(ls.length(), 0);
    Ok(())
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

async fn web_ec_key_pair(key_type: &str, uses: &[&str]) -> CryptoKeyPair {
    let subtle = window().crypto().unwrap().subtle();
    let params = web_sys::EcKeyGenParams::new(key_type, "P-256");
    let usages = js_array(uses);
    let promise = subtle
        .generate_key_with_object(&params, true, &usages)
        .unwrap();
    let key_pair = wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
    CryptoKeyPair::from(key_pair)
}
*/
