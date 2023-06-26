//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use wasm_bindgen_test::wasm_bindgen_test_configure;
use wasm_bindgen_test::*;

extern crate tomb_wasm;
use tomb_wasm::Tomb;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn pass() {
    assert_eq!(1 + 1, 2);
}

#[cfg(test)]
pub async fn helper_method_example() -> Tomb {
    let tomb = Tomb::new(
        "http://test.tomb.local".to_string(),
        "long-secure-token-here".to_string()
    ).await.unwrap();
    tomb
}

#[wasm_bindgen_test]
pub async fn test_tomb() {
    // calling a setup function.
    let tomb = helper_method_example().await;
    let buckets = tomb.buckets().unwrap();
    // TODO: More in depth assertions
}