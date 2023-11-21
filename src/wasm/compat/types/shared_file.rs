use crate::prelude::filesystem::sharing::SharedFile;
use std::ops::Deref;
use wasm_bindgen::prelude::wasm_bindgen;
use wnfs::private::share::SharePayload;

#[wasm_bindgen]
pub struct WasmSharedFile(pub(crate) SharedFile);

#[wasm_bindgen]
impl WasmSharedFile {
    // pub fn payload(&self) -> SharePayload {
    //     self.0.payload
    // }

    #[wasm_bindgen(js_name = "mimeType")]
    pub fn mime_type(&self) -> String {
        self.0.mime_type.clone()
    }

    pub fn size(&self) -> String {
        self.0.size.to_string()
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
