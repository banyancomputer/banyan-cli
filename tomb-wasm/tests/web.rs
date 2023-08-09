//! Test suite for the Web and headless browsers.

extern crate wasm_bindgen_test;
use gloo::console::log;
use gloo::utils::window;
use js_sys::{Array, Reflect};
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::wasm_bindgen_test_configure;
use wasm_bindgen_test::*;
use web_sys::CryptoKey;

extern crate tomb_wasm;

wasm_bindgen_test_configure!(run_in_browser);

#[cfg(test)]
fn js_array(values: &[&str]) -> JsValue {
    return JsValue::from(
        values
            .iter()
            .map(|x| JsValue::from_str(x))
            .collect::<Array>(),
    );
}

#[cfg(test)]
async fn ec_key(key_type: &str, uses: &[&str]) -> CryptoKey {
    let subtle = window().crypto().unwrap().subtle();
    let params = web_sys::EcKeyGenParams::new(key_type, "P-256");
    let usages = js_array(uses);
    let future = subtle
        .generate_key_with_object(&params, true, &usages)
        .unwrap();
    let key_pair = wasm_bindgen_futures::JsFuture::from(future).await.unwrap();
    let private_key = Reflect::get(&key_pair, &tomb_wasm::value!("privateKey"))
        .unwrap()
        .dyn_into::<web_sys::CryptoKey>()
        .unwrap();
    private_key
}

#[wasm_bindgen_test]
async fn tomb_wasm() {
    log!("tomb_wasm_test: new");
    // Test initialization
    let api_key = ec_key("ECDSA", &["sign", "verify"]).await;
    let mut tomb = tomb_wasm::TombWasm::new(
        api_key,
        "account-identifier".to_string(),
        "https://api.tomb-demo.org".to_string(),
    );

    // Test API calls
    log!("tomb_wasm_test: account metadata");
    let _storage = tomb.get_total_storage().await.unwrap();
    let _trash = tomb.get_trash_bucket().await.unwrap();
    let _buckets = tomb.get_buckets().await.unwrap();
    let _snapshots = tomb.get_snapshots().await.unwrap();

    // Test using bucket's API endpoint
    log!("tomb_wasm_test: bucket metadata");
    let bucket_id = "bucket-identifier";
    let _bucket_storage = tomb.get_bucket_storage(bucket_id).await.unwrap();
    let _bucket_keys = tomb.get_bucket_keys(bucket_id).await.unwrap();
    let _bucket_snapshots = tomb.get_bucket_snapshots(bucket_id).await.unwrap();

    // Test loading and interacting with a bucket
    log!("tomb_wasm_test: bucket interaction");
    tomb.load(bucket_id).await.unwrap();
    let wrapping_key = ec_key("ECDH", &["deriveBits"]).await;
    tomb.unlock(bucket_id, wrapping_key)
        .await
        .unwrap();
    let _ = tomb.ls(bucket_id, "/", None).await.unwrap();
    let _ = tomb.ls(bucket_id, "/", Some("1".into())).await.unwrap();
}
