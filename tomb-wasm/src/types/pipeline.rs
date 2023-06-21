use tomb_common::types::pipeline::Manifest as TombManifeset;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
#[derive(Default)]
pub struct Manifest(pub(crate) TombManifeset);
