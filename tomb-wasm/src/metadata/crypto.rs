use crate::fetch::http::get_data;
use tomb_crypt::prelude::*;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

#[allow(missing_debug_implementations)]
#[wasm_bindgen]
pub struct MyPrivateKey(pub(crate) EcEncryptionKey);

#[wasm_bindgen]
impl MyPrivateKey {
    #[wasm_bindgen]
    pub async fn new(url: String) -> Result<MyPrivateKey, JsValue> {
        use tomb_crypt::prelude::*;
        let data = get_data(url).await.unwrap();
        let key = EcEncryptionKey::import(&data).await.unwrap();
        Ok(Self(key))
    }
}

#[cfg(test)]
mod test {
    use crate::metadata::crypto::MyPrivateKey;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn load_key() {
        let url = "https://gist.githubusercontent.com/organizedgrime/f292f28a6ea39cea5fd1b844c51da4fb/raw/wrapping_key.pem".to_string();
        assert!(MyPrivateKey::new(url).await.is_ok());
    }
}
