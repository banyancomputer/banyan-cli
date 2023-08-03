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
async fn private_crypto_key() -> CryptoKey {
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

#[wasm_bindgen_test]
async fn tomb_new() {
    // calling a setup function.
    let crypto_key = private_crypto_key().await;
    let tomb = tomb_wasm::TombWasm::new(crypto_key, "http://echo.jsontest.com".to_string());
}
