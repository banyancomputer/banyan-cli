use base64::{engine::general_purpose, Engine as _};
use js_sys::{Array, Error};
use std::fmt::Debug;
use tomb_common::crypto::rsa::RsaPrivateKey;
use wasm_bindgen::prelude::*;

pub type JsResult<T> = Result<T, js_sys::Error>;

/// Turn a value into a JsValue
#[macro_export]
macro_rules! value {
    ($value:expr) => {
        wasm_bindgen::JsValue::from($value)
    };
}

#[wasm_bindgen(js_name = "setPanicHook")]
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[allow(dead_code)]
pub(crate) fn error<E>(message: &str) -> impl FnOnce(E) -> Error + '_
where
    E: Debug,
{
    move |e| Error::new(&format!("{message}: {e:?}"))
}

#[allow(dead_code)]
pub(crate) fn anyhow_error<E>(message: &str) -> impl FnOnce(E) -> anyhow::Error + '_
where
    E: Debug,
{
    move |e| anyhow::Error::msg(format!("{message}: {e:?}"))
}

pub(crate) fn map_to_rust_vec<T, F: FnMut(JsValue) -> JsResult<T>>(
    array: &Array,
    f: F,
) -> JsResult<Vec<T>> {
    array
        .to_vec()
        .into_iter()
        .map(f)
        .collect::<JsResult<Vec<_>>>()
}

#[inline]
#[allow(dead_code)]
/// Convert Vec of bytes to JsResult of bytes with known length
pub(crate) fn expect_bytes<const N: usize>(bytes: Vec<u8>) -> JsResult<[u8; N]> {
    bytes.try_into().map_err(|v: Vec<u8>| {
        Error::new(&format!(
            "Unexpected number of bytes received. Expected {N}, but got {}",
            v.len()
        ))
    })
}

#[allow(dead_code)]
pub(crate) fn convert_path_segments(path_segments: &Array) -> JsResult<Vec<String>> {
    map_to_rust_vec(path_segments, |v| {
        v.as_string()
            .ok_or_else(|| Error::new("Invalid path segments: Expected an array of strings"))
    })
}

#[allow(dead_code)]
pub(crate) fn string_to_rsa_key(crypto_key: JsValue) -> JsResult<RsaPrivateKey> {
    // Read the JsValue as a string
    let crypto_key = crypto_key
        .as_string()
        .ok_or_else(|| Error::new("Invalid crypto key: Expected a string"))?;
    let key_bytes = general_purpose::STANDARD
        .decode(crypto_key.as_bytes())
        .map_err(|_| Error::new("Invalid crypto key: Expected a valid base64 string"))?;
    let key = RsaPrivateKey::from_der(&key_bytes)
        .map_err(|_| Error::new("Invalid crypto key: Expected a valid RSA private key"))?;
    Ok(key)
}

#[cfg(test)]
mod test {
    use super::*;
    use tomb_common::crypto::rsa::{ExchangeKey, PrivateKey};
    use wasm_bindgen_test::wasm_bindgen_test_configure;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_string_to_rsa_key() {
        const PKCS8_STRING: &str = "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";
        let plaintext = b"Hello, world!";
        let priv_key = string_to_rsa_key(value!(PKCS8_STRING)).unwrap();
        let pub_key = priv_key.get_public_key();
        let ciphertext = pub_key.encrypt(plaintext).await.unwrap();
        let decrypted = priv_key.decrypt(&ciphertext).await.unwrap();
        assert_eq!(plaintext, &decrypted[..]);
    }
}
