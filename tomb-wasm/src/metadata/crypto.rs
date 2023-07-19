use tomb_crypt::prelude::{EcEncryptionKey, WrappingPrivateKey};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use crate::fetch::http::get_data;

#[allow(missing_debug_implementations)]
#[wasm_bindgen]
pub struct PrivateKey(pub(crate) EcEncryptionKey);

#[wasm_bindgen]
impl PrivateKey {
    #[wasm_bindgen]
    pub async fn new(url: String) -> Result<PrivateKey, JsValue> {
        let data = get_data(url.clone()).await.unwrap();
        let pem = pem::parse(data).unwrap();
        let key = EcEncryptionKey::import(pem.contents()).await.unwrap();
        Ok(Self(key))
    }
}

#[cfg(test)]
mod test {
    use crate::metadata::crypto::PrivateKey;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn load_key() {
        let url = "https://gist.githubusercontent.com/organizedgrime/f292f28a6ea39cea5fd1b844c51da4fb/raw/wrapping_key.pem".to_string();
        assert!(PrivateKey::new(url).await.is_ok());
    }
}
