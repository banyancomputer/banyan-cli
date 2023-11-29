use crate::prelude::filesystem::sharing::SharedFile;
use crate::wasm::{TombResult, TombWasmError};
use std::ops::Deref;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct WasmSharedFile(pub(crate) SharedFile);

#[wasm_bindgen]
impl WasmSharedFile {
    pub fn export_b64_url(&self) -> TombResult<String> {
        Ok(self
            .0
            .export_b64_url()
            .map_err(|_| TombWasmError(format!("Unable to export shared file as b64 url data")))?)
    }

    pub fn import_b64_url(b64_string: String) -> TombResult<WasmSharedFile> {
        Ok(WasmSharedFile(
            SharedFile::import_b64_url(b64_string).map_err(|_| {
                TombWasmError(format!("Unable to import shared file from b64 url data"))
            })?,
        ))
    }

    #[wasm_bindgen(js_name = "mimeType")]
    pub fn mime_type(&self) -> Option<String> {
        self.0.mime_type.clone()
    }

    pub fn size(&self) -> Option<String> {
        self.0.size.map(|value| value.to_string())
    }

    #[wasm_bindgen(js_name = "fileName")]
    pub fn file_name(&self) -> String {
        self.0.file_name.clone()
    }
}

impl Deref for WasmSharedFile {
    type Target = SharedFile;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<WasmSharedFile> for SharedFile {
    fn from(wasm_bucket_key: WasmSharedFile) -> Self {
        wasm_bucket_key.0
    }
}
