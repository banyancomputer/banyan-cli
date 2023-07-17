// use base64::engine::general_purpose::STANDARD as B64;
// use base64::Engine;

use web_sys::{SubtleCrypto, CryptoKeyPair, CryptoKey, EcKeyGenParams};
use js_sys::{JsString, Promise, Uint8Array, Array, Error as JsError, Object, Reflect, ArrayBuffer};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen::prelude::*;
use gloo::utils::window;

use crate::key_seal::common::{AES_KEY_SIZE, ECDH_SECRET_BYTE_SIZE, FINGERPRINT_SIZE, SALT_SIZE};
use crate::key_seal::{pretty_fingerprint, KeySealError};

/* Wasm Utilities */

fn js_array(values: &[&str]) -> JsValue {
    return JsValue::from(values.into_iter()
        .map(|x| JsValue::from_str(x))
        .collect::<Array>());
}

pub(crate) type JsResult<T> = Result<T, JsError>;

/* Subtle Crypto Utilities*/

/// Get the crypto object from the window
pub(crate) fn crypto() -> Result<web_sys::Crypto, KeySealError> {
    window().crypto().map_err(|err| {
        KeySealError::crypto_unavailable(err.into())
    })
}

/// Get the subtle crypto object from the window
pub(crate) fn subtle_crypto() -> Result<SubtleCrypto, KeySealError> {
    let crypto = crypto()?;
    Ok(crypto.subtle())
}

/// Get the private key from a key pair
pub(crate) fn private_key(key_pair: &CryptoKeyPair) -> CryptoKey {
    Reflect::get(key_pair, &JsString::from("privateKey")).unwrap().into()
}

/// Get the public key from a key pair
pub(crate) fn public_key(key_pair: &CryptoKeyPair) -> CryptoKey {
    Reflect::get(key_pair, &JsString::from("publicKey")).unwrap().into()
}

/// Generate a new EC encryption key pair 
pub(crate) async fn generate_ec_encryption_key_pair() -> JsResult<CryptoKeyPair> {
    let params =
        EcKeyGenParams::new("ECDH", "P-384");
    let usages = js_array(&["deriveBits"]);

    let crypto = subtle_crypto()?;

    let key_pair_promise: Promise = crypto.generate_key_with_object(
        &params,
        true,
        &usages,
    ).expect("key pair generation to start");

    let key_pair = JsFuture::from(key_pair_promise).await.expect("key pair generation to succeed");

    Ok(CryptoKeyPair::from(key_pair))
}

/// Import an ec key from a &[u8] in der format
pub(crate) async fn import_ec_key_der(format: &str, der_bytes: &[u8]) -> JsResult<CryptoKey> {
    let crypto = subtle_crypto()?;
    let import_promise = crypto.import_key_with_object(
        format,
        &Uint8Array::from(der_bytes),
        &EcKeyGenParams::new("ECDH", "P-384"),
        true,
        &js_array(&["deriveBits"]),
    ).expect("import promise to be created");
    let import_result = JsFuture::from(import_promise).await.expect("import promise to succeed");
    Ok(import_result.into())
}

/// Export an ec key as a Vec<u8> in der format
pub(crate) async fn export_ec_key_der(format: &str, public_key: &CryptoKey) -> JsResult<Vec<u8>> {
    let crypto = subtle_crypto()?;
    let export_promise = crypto.export_key(
        format,
        &public_key,
    ).expect("export promise to be created");
    let export_result = JsFuture::from(export_promise).await.expect("export promise to succeed");
    let export_result = export_result.dyn_into::<ArrayBuffer>().expect("export result to be an array buffer");
    let export_result = Uint8Array::new(&export_result).to_vec();
    Ok(export_result)
}

pub(crate) async fn fingerprint_public_ec_key(public_key: &CryptoKey) -> JsResult<[u8; FINGERPRINT_SIZE]> {
    let public_key_bytes = export_ec_key_der("raw", public_key).await?;

    // Note (amiller68): This only works for P-384 keys
    let size = 49;
    let mut compressed_point = vec![0u8; size];
    let x = public_key_bytes[1..size].to_vec();
    let y = public_key_bytes[size..].to_vec();

    compressed_point[0] = if y[y.len() - 1] % 2 == 0 { 0x02 } else { 0x03 };
    compressed_point[1..].copy_from_slice(&x);

    let crypto = subtle_crypto()?;
    let digest_promise = crypto.digest_with_str_and_u8_array(
        "SHA-1",
        compressed_point.as_mut_slice(), 
    ).expect("digest promise to be created");

    let digest_result = JsFuture::from(digest_promise).await.expect("digest promise to succeed");

    let digest_result = digest_result.dyn_into::<ArrayBuffer>().expect("digest result to be an array buffer");

    let digest_result = Uint8Array::new(&digest_result).to_vec();

    let mut fingerprint = [0u8; FINGERPRINT_SIZE];
    fingerprint.copy_from_slice(&digest_result);

    Ok(fingerprint)

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


//     openssl::sha::sha1(&public_key_bytes)
// }


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
