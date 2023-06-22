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
pub fn setup_method_example() -> Tomb {
    let mut tomb = Tomb::new();
    tomb.setup_method();
    tomb
}

#[wasm_bindgen_test]
pub fn test_tomb() {
    // calling a setup function.
    let input_tomb = setup_method_example();
    assert!(input_tomb.is_setup());
}