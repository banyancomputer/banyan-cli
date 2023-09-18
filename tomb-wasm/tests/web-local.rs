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


#[wasm_bindgen_test]
async fn setup_fs() -> TombResult<()> {
    log!("tomb_wasm_test: get_usage()");

    



    Ok(())
}