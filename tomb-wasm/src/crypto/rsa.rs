use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CryptoKey, CryptoKeyPair,
    RsaOaepParams
};
use js_sys::{
    Uint8Array, Array, Reflect
};
use gloo::{
    utils::window,
    console::log
};

#[allow(dead_code)]
pub async fn gen_key_pair(
    key_usages: &[&str]
) -> Result<CryptoKeyPair, JsValue> {
    log!("tomb-wasm: gen_key_pair()");
    let key_usages = JsValue::from(key_usages.into_iter()
        .map(|x| JsValue::from_str(x))
        .collect::<Array>());
    let crypto = window().crypto().unwrap_throw();
    let params = RsaOaepParams::new("SHA-256");
    let key_pair = crypto.subtle().generate_key_with_object(
        &params,
        true,
        &key_usages
    )?;
    let key_pair = JsFuture::from(key_pair).await.expect("gen_key_pair() failed");
    Ok(key_pair.into())
}

#[allow(dead_code)]
pub async fn encrypt(
    key_pair: &CryptoKeyPair,
    data: Uint8Array
) -> Result<JsValue, JsValue> {
    log!("tomb-wasm: encrypt()");
    let key: CryptoKey = Reflect::get(&key_pair, &JsValue::from("publicKey"))
        .expect("privateKey").into();
    let crypto = window().crypto().unwrap_throw();
    let params = RsaOaepParams::new("SHA-256");
    let encrypted = crypto.subtle().encrypt_with_object_and_buffer_source(
        &params,
        &key,
        data.as_ref()
    )?;
    let encrypted = JsFuture::from(encrypted).await.expect("encrypt() failed");
    Ok(encrypted.into())
}

#[allow(dead_code)]
pub async fn decrypt(
    key_pair: &CryptoKeyPair,
    data: Uint8Array
) -> Result<Uint8Array, JsValue> {
    log!("tomb-wasm: decrypt()");
    let key: CryptoKey = Reflect::get(&key_pair, &JsValue::from("privateKey"))
        .expect("privateKey").into();
    let crypto = window().crypto().unwrap_throw();
    let params = RsaOaepParams::new("SHA-256");
    let decrypted = crypto.subtle().decrypt_with_object_and_buffer_source(
        &params,
        &key,
        data.as_ref()
    )?;
    let decrypted = JsFuture::from(decrypted).await.expect("decrypt() failed");
    Ok(decrypted.into())
}


#[cfg(test)]
mod tests {
    use js_sys::Uint8Array;
    use wasm_bindgen_test::wasm_bindgen_test_configure;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    // TODO: Get these tests working

    // #[wasm_bindgen_test]
    // async fn test_gen_key_pair() {
    //     let key_pair = super::gen_key_pair(
    //         &["encrypt", "decrypt"]
    //     ).await.unwrap();
    //     assert!(key_pair.is_object());
    // }

    
    // #[wasm_bindgen_test]
    // async fn test_rsa_encrypt() {
    //     let key_pair = super::gen_key_pair(
    //         &["encrypt", "decrypt"]
    //     ).await.unwrap();
    //     let data = Uint8Array::from(vec![0; 32].as_slice());
    //     let encrypted = super::encrypt(&key_pair, data).await.unwrap();
    //     assert!(encrypted.is_object());
    // }

    // #[wasm_bindgen_test]
    // async fn test_rsa_decrypt() {
    //     let key_pair = super::gen_key_pair(
    //         &["encrypt", "decrypt"]
    //     ).await.unwrap();
    //     let data = Uint8Array::from(vec![0; 32].as_slice());
    //     let encrypted = super::encrypt(&key_pair, data.clone()).await.unwrap();
    //     let decrypted = super::decrypt(&key_pair, encrypted.into()).await.unwrap();
    //     assert!(decrypted.is_object());
    //     assert_eq!(decrypted.as_string().unwrap(), data.as_string().unwrap());
    // }
}