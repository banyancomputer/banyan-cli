
use tomb_common::crypto::rsa::RsaPrivateKey as TombPrivateKey;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::fetch::http::get_data;

#[wasm_bindgen]
pub struct PrivateKey(pub(crate) TombPrivateKey);

#[wasm_bindgen]
impl PrivateKey {
    #[wasm_bindgen]
    pub async fn new(url: String) -> Result<PrivateKey, JsValue> {
        let data = get_data(url.clone()).await.unwrap();
        let pem = pem::parse(data).unwrap();
        let key = TombPrivateKey::from_der(pem.contents()).unwrap();
        Ok(Self(key))
    }
}

#[cfg(test)]
mod test {
    use wasm_bindgen_test::wasm_bindgen_test;
    use crate::metadata::crypto::PrivateKey;

    #[wasm_bindgen_test]
    async fn load_key() {
        let url = "https://gist.githubusercontent.com/organizedgrime/f292f28a6ea39cea5fd1b844c51da4fb/raw/f9db2c3ff12e64fa6bbc291666081212365be749/wrapping_key.pem".to_string();
        assert!(PrivateKey::new(url).await.is_ok());
    }
}