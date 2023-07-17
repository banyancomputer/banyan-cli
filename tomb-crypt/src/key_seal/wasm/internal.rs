// use base64::engine::general_purpose::STANDARD as B64;
// use base64::Engine;

use web_sys::{SubtleCrypto, CryptoKeyPair, EcKeyGenParams};
use js_sys::{JsString, Promise, Uint8Array, Array};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::prelude::*;
use gloo::utils::window;

use super::KeySealError;

// use crate::key_seal::common::{AES_KEY_SIZE, ECDH_SECRET_BYTE_SIZE, FINGERPRINT_SIZE, SALT_SIZE};
// use crate::key_seal::{pretty_fingerprint, KeySealError};

pub(crate) type JsResult<T> = Result<T, JsValue>;

fn js_array(values: &[&str]) -> JsValue {
    return JsValue::from(values.into_iter()
        .map(|x| JsValue::from_str(x))
        .collect::<Array>());
}

pub(crate) fn subtle_crypto() -> Result<SubtleCrypto, KeySealError> {
    let crypto = window().crypto().map_err(|err| {
        KeySealError::subtle_crypto_unavailable(err)
    });
    Ok(crypto?.subtle())
}

// pub(crate) fn base64_decode(data: &str) -> JsResult<Vec<u8>> {
//     B64.decode(data).map_err(KeySealError::bad_base64)
// }

// pub(crate) fn base64_encode(data: &[u8]) -> String {
//     B64.encode(data)
// }

// pub(crate) fn ecdh_exchange(
//     private: &PKey<Private>,
//     public: &PKey<Public>,
// ) -> [u8; ECDH_SECRET_BYTE_SIZE] {
//     let mut deriver =
//         Deriver::new(private).expect("creation of ECDH derivation context from private key");
//     deriver
//         .set_peer(public)
//         .expect("setting openssl peer for ECDH key derivation");

//     let calculated_bytes = deriver
//         .derive_to_vec()
//         .expect("calculate common public key");

//     let mut key_slice = [0u8; ECDH_SECRET_BYTE_SIZE];
//     key_slice.copy_from_slice(&calculated_bytes);

//     key_slice
// }

// pub(crate) fn fingerprint(public_key: &PKey<Public>) -> [u8; FINGERPRINT_SIZE] {
//     let ec_group = ec_group();
//     let mut big_num_context = BigNumContext::new().expect("BigNumContext creation");

//     let ec_public_key = public_key.ec_key().expect("key to be an EC derived key");

//     let public_key_bytes = ec_public_key
//         .public_key()
//         .to_bytes(
//             &ec_group,
//             PointConversionForm::COMPRESSED,
//             &mut big_num_context,
//         )
//         .expect("generate public key bytes");

//     openssl::sha::sha1(&public_key_bytes)
// }

pub(crate) async fn generate_ec_key() -> JsResult<CryptoKeyPair> {
    let params =
        EcKeyGenParams::new("ECDH", "P-384");
    let usages = js_array(&["deriveKey", "deriveBits"]);
    let crypto = subtle_crypto()?;
    // let key_pair_promise: CryptoKeyPair = crypto.generate_key_with_object(
    //     &params,
    //     true,
    //     &usages,
    // )?;
    // let key_pair = JsFuture::from(key_pair_promise).await?;
    // let key_pair = key_pair.dyn_into::<CryptoKeyPair>()?;
    // Ok(key_pair)
    todo!()
}

// pub(crate) fn generate_info(encryptor: &[u8], decryptor: &[u8]) -> String {
//     format!(
//         "use=key_seal,encryptor={},decryptor={}",
//         pretty_fingerprint(encryptor),
//         pretty_fingerprint(decryptor),
//     )
// }

// pub(crate) fn hkdf(secret_bytes: &[u8], info: &str) -> ([u8; SALT_SIZE], [u8; AES_KEY_SIZE]) {
//     let mut salt = [0u8; SALT_SIZE];
//     openssl::rand::rand_bytes(&mut salt).expect("unable to generate random IV");
//     (salt, hkdf_with_salt(secret_bytes, &salt, info))
// }

// pub(crate) fn hkdf_with_salt(secret_bytes: &[u8], salt: &[u8], info: &str) -> [u8; AES_KEY_SIZE] {
//     let mut expanded_key = [0; AES_KEY_SIZE];

//     openssl_hkdf::hkdf::hkdf(
//         MessageDigest::sha256(),
//         secret_bytes,
//         salt,
//         info.as_bytes(),
//         &mut expanded_key,
//     )
//     .expect("hkdf operation to succeed");

//     expanded_key
// }

// pub(crate) fn public_from_private(private_key: &PKey<Private>) -> PKey<Public> {
//     let ec_group = ec_group();

//     let ec_key = private_key
//         .ec_key()
//         .expect("unable to extract EC private key from private key");
//     // Have to do a weird little dance here as to we need to temporarily go into bytes to get to a
//     // public only key
//     let pub_ec_key: EcKey<Public> = EcKey::from_public_key(&ec_group, ec_key.public_key())
//         .expect("unable to turn public key bytes into public key");

//     PKey::from_ec_key(pub_ec_key).expect("unable to wrap public key in common struct")
// }

// pub(crate) fn unwrap_key(secret_bytes: &[u8], protected_key: &[u8]) -> [u8; AES_KEY_SIZE] {
//     let wrapping_key =
//         AesKey::new_decrypt(secret_bytes).expect("use of secret bytes when unwrapping key");

//     let mut plaintext_key = [0u8; AES_KEY_SIZE];
//     aes::unwrap_key(&wrapping_key, None, &mut plaintext_key, protected_key)
//         .expect("unwrapping to succeed");

//     plaintext_key
// }

// pub(crate) fn wrap_key(secret_bytes: &[u8], unprotected_key: &[u8]) -> [u8; AES_KEY_SIZE + 8] {
//     let wrapping_key =
//         AesKey::new_encrypt(secret_bytes).expect("use of secret bytes when wrapping key");

//     let mut enciphered_key = [0u8; AES_KEY_SIZE + 8];
//     aes::wrap_key(&wrapping_key, None, &mut enciphered_key, unprotected_key)
//         .expect("wrapping to succeed");

//     enciphered_key
// }
