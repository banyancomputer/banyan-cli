use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
#[derive(Default)]
pub struct Manifest(pub(crate) tomb_common::types::pipeline::Manifest);
