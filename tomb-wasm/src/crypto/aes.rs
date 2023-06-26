use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CryptoKey, 
    AesKeyGenParams
};
use js_sys::{
    Uint8Array, Array
};
use gloo::{
    utils::window,
    console::log
};

// Note: None of this stuff is WASM bound, so don't bother with #[wasm_bindgen] or correct return types

#[allow(dead_code)]
pub async fn import_key_from_bytes(
    algorithm: &str,
    extractable: bool, 
    key_usages: &[&str],
    key_data: &[u8]
) -> Result<CryptoKey, JsValue> {
    log!("tomb-wasm: key_from_vec()");
    let key_usages = JsValue::from(key_usages.into_iter()
        .map(|x| JsValue::from_str(x))
        .collect::<Array>());
    let params = AesKeyGenParams::new(algorithm, 256);
    let key = JsFuture::from(window()
        .crypto()?
        .subtle()
        .import_key_with_object(
            "raw",
            &js_sys::Uint8Array::from(key_data).buffer(),
            &params,
            extractable,
            &key_usages
        )?).await.expect("key_from_vec() failed");
    Ok(key.dyn_into::<CryptoKey>().unwrap_throw())
}

// TODO: Do I need to add a delimeter between the IV and Ciphertext?
#[allow(dead_code)]
pub async fn gcm_encrypt(
    key: &CryptoKey,
    data: Uint8Array
) -> Result<JsValue, JsValue> {
    let crypto = window().crypto().unwrap_throw();
    let iv = crypto.get_random_values_with_u8_array(&mut [0; 16])?;
    let params = web_sys::AesGcmParams::new("AES-GCM", &iv);
    let encrypted = crypto.subtle().encrypt_with_object_and_buffer_source(
        &params,
        key,
        data.as_ref()
    )?;
    let encrypted = JsFuture::from(encrypted).await.expect("encrypt() failed");
    Ok(encrypted.into())
}

// TODO: Do I need to account for the delimiter between the IV and Ciphertext?
// TODO: This is raising the following error:
//     wasm-bindgen: imported JS function that was not marked as `catch` threw an error: expected a number argument
#[allow(dead_code)]
pub async fn gcm_decrypt(
    key: &CryptoKey,
    data: Uint8Array
) -> Result<JsValue, JsValue> {
    log!("tomb-wasm: gcm_decrypt()");
    let crypto = window().crypto().unwrap_throw();
    // Read the iv from the data
    let iv_data_clone = data.clone();
    let iv = iv_data_clone.slice(0, 16);
    // Read the ciphertext from the data
    let ciphertext_clone = data.clone();
    log!("tomb-wasm: gcm_decrypt() - reading ciphertext");
    let data = ciphertext_clone.slice(16, data.length());
    // Create the params object
    log!("tomb-wasm: gcm_decrypt() - creating params object");
    let params = web_sys::AesGcmParams::new("AES-GCM", &iv);
    // Decrypt the data
    log!("tomb-wasm: gcm_decrypt() - decrypting data");
    let decrypted = crypto.subtle().decrypt_with_object_and_buffer_source(
        &params,
        key,
        data.as_ref()
    )?;
    // Await the promise and return the decrypted data
    let decrypted = JsFuture::from(decrypted).await.expect("decrypt() failed");
    Ok(decrypted.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use js_sys::Uint8Array;
    use wasm_bindgen_test::wasm_bindgen_test_configure;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_import_key_from_bytes() {
        let key_data: Vec<u8> = vec![0; 32];
        let key = super::import_key_from_bytes(
            "AES-GCM", 
            true, 
            &["encrypt", "decrypt"], 
            &key_data
        ).await.unwrap();
        // TODO: More informative assertions
        assert!(key.extractable());
        assert_eq!(key.usages().length(), 2);
    }

    #[wasm_bindgen_test]
    async fn test_gcm_encrypt() {
        let key_data: Vec<u8> = vec![0; 32];
        let key = super::import_key_from_bytes(
            "AES-GCM", 
            true, 
            &["encrypt", "decrypt"], 
            &key_data
        ).await.unwrap();
        let data = Uint8Array::from(vec![0; 32].as_slice());
        let encrypted = super::gcm_encrypt(&key, data).await.unwrap();
        assert!(encrypted.is_object());
    }

    // TODO: Get this test working
    // #[wasm_bindgen_test]
    // async fn test_gcm_decrypt() {
    //     set_panic_hook();
    //     let key_data: Vec<u8> = vec![0; 32];
    //     log!("tomb-wasm: test_gcm_decrypt()");
    //     let key = super::import_key_from_bytes(
    //         "AES-GCM", 
    //         true, 
    //         &["encrypt", "decrypt"], 
    //         &key_data
    //     ).await.unwrap();
    //     log!("tomb-wasm: test_gcm_decrypt() - key imported");
    //     let data = Uint8Array::from(vec![0; 32].as_slice());
    //     let encrypted = super::gcm_encrypt(&key, data.clone()).await.unwrap();
    //     log!("tomb-wasm: test_gcm_decrypt() - data encrypted");
    //     let decrypted = super::gcm_decrypt(&key, encrypted.into()).await.unwrap();
    //     log!("tomb-wasm: test_gcm_decrypt() - data decrypted");
    //     assert!(decrypted.is_object());
    //     log!("tomb-wasm: test_gcm_decrypt() - assertions passed");
    //     assert_eq!(decrypted.as_string().unwrap(), data.as_string().unwrap());
    // }
}