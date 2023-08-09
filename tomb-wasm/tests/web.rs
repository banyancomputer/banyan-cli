//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
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
async fn encryption_key() -> CryptoKey {
    let subtle = window().crypto().unwrap().subtle();
    let params = web_sys::EcKeyGenParams::new("ECDH", "P-256");
    let usages = js_array(&["deriveBits"]);
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

#[cfg(test)]
async fn api_key() -> CryptoKey {
    let subtle = window().crypto().unwrap().subtle();
    let params = web_sys::EcKeyGenParams::new("ECDSA", "P-256");
    let usages = js_array(&["sign", "verify"]);
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
async fn tomb_new() {
    let subtle = window().crypto().unwrap().subtle();
    let params = web_sys::EcKeyGenParams::new("ECDH", "P-256");
    let usages = js_array(&["deriveBits"]);
    let future = subtle
        .generate_key_with_object(&params, true, &usages)
        .unwrap();
    let key_pair = wasm_bindgen_futures::JsFuture::from(future).await.unwrap();
    let encryption_key = Reflect::get(&key_pair, &tomb_wasm::value!("privateKey"))
        .unwrap()
        .dyn_into::<web_sys::CryptoKey>()
        .unwrap();
    // private_key
    // calling a setup function.
    // let encryption_key = encryption_key().await;
    let api_key = api_key().await;
    let mut tomb = tomb_wasm::TombWasm::new(
        encryption_key,
        api_key,
        "account-identifier".to_string(),
        "https://api.tomb-demo.org".to_string(),
    );

    tomb.load_bucket("bucket-identifier")
        .await
        .unwrap();

    let future = subtle
        .generate_key_with_object(&params, true, &usages)
        .unwrap();
    let key_pair = wasm_bindgen_futures::JsFuture::from(future).await.unwrap();
    let encryption_key = Reflect::get(&key_pair, &tomb_wasm::value!("privateKey"))
        .unwrap()
        .dyn_into::<web_sys::CryptoKey>()
        .unwrap();

    tomb.unlock_bucket("bucket-identifier", encryption_key)
        .await
        .unwrap();

    tomb.ls_bucket("bucket-identifier", "/").await.unwrap();
}
