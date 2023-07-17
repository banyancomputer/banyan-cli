// use base64::engine::general_purpose::STANDARD as B64;
// use base64::Engine;

use web_sys::{SubtleCrypto, CryptoKeyPair, CryptoKey, EcKeyGenParams, HkdfParams};
use js_sys::{JsString, Promise, Uint8Array, Array, Error as JsError, Reflect, ArrayBuffer, Object};
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

/// Run an Async function that returns a promise. Return as a Vec<u8>


async fn crypto_method(
    method: Result<Promise, JsValue>
) -> JsResult<JsValue> {
    Ok(JsFuture::from(method?).await.expect("promise to succeed"))
}

// let crypto = subtle_crypto()?;
//     let digest_promise = crypto.digest_with_str_and_u8_array(
//         "SHA-1",
//         compressed_point.as_mut_slice(), 
//     ).expect("digest promise to be created");

//     let digest_result = JsFuture::from(digest_promise).await.expect("digest promise to succeed");

//     let digest_result = digest_result.dyn_into::<ArrayBuffer>().expect("digest result to be an array buffer");

//     let digest_result = Uint8Array::new(&digest_result).to_vec();

/* Key getters, exporters, and importers */

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
    let key_pair = crypto_method(crypto.generate_key_with_object(
        &params,
        true,
        &usages,
    )).await?;
    Ok(CryptoKeyPair::from(key_pair))
}

/// Import an ec key from a &[u8] in der format
pub(crate) async fn import_ec_key_der(format: &str, der_bytes: &[u8]) -> JsResult<CryptoKey> {
    let crypto = subtle_crypto()?;
    let import = crypto_method(crypto.import_key_with_object(
        format,
        &Uint8Array::from(der_bytes),
        &EcKeyGenParams::new("ECDH", "P-384"),
        true,
        &js_array(&["deriveBits"]),
    )).await?;
    Ok(import.into())
}

/// Import a raw AES GCM key from a CryptoKey
async fn import_aes_key(
    key_data: &Object,
    name: &str,
    uses: &[&str],
) -> JsResult<CryptoKey> {
    let crypto = subtle_crypto()?;
    let import = crypto_method(crypto.import_key_with_str(
        "raw",
        &key_data,
        name,
        true,
        &js_array(uses),
    )).await?;
    Ok(import.into())
}

/// Import an ec key from a &[u8] in pem format
pub(crate) async fn import_ec_key_pem(format: &str, pem_bytes: &[u8]) -> JsResult<CryptoKey> {
   todo!()
}

/// Export an ec key as a Vec<u8> in der format
pub(crate) async fn export_ec_key_der(format: &str, public_key: &CryptoKey) -> JsResult<Vec<u8>> {
    let crypto = subtle_crypto()?;
    let export = crypto_method(crypto.export_key(
        format,
        &public_key,
    )).await?;
    let export= export.dyn_into::<ArrayBuffer>().expect("export result to be an array buffer");
    let export= Uint8Array::new(&export).to_vec();
    Ok(export)
}

/// Export an ec key as a Vec<u8> in pem format
pub(crate) async fn export_ec_key_pem(format: &str, public_key: &CryptoKey) -> JsResult<Vec<u8>> {
    todo!()
}

/// Fingerprint an ec public key
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
    let digest = crypto_method(
        crypto.digest_with_str_and_u8_array(
            "SHA-1",
            compressed_point.as_mut_slice(), 
        )
    ).await?;

    let digest = digest.dyn_into::<ArrayBuffer>().expect("digest result to be an array buffer");

    let digest = Uint8Array::new(&digest).to_vec();

    let mut fingerprint = [0u8; FINGERPRINT_SIZE];
    fingerprint.copy_from_slice(&digest);

    Ok(fingerprint)
}

/* Key Derivation and Wrapping Utilities  */

/// Derive am HKDF key from a secret.
pub(crate) async fn derive_hkdf_key(secret_bytes: &[u8], info: &str) -> JsResult<([u8; SALT_SIZE], CryptoKey)> {
    let mut salt = [0u8; SALT_SIZE];
    let crypto = crypto()?;
    crypto.get_random_values_with_u8_array(&mut salt).expect("unable to generate random IV");
    Ok((salt, hkdf_with_salt(secret_bytes, &salt, info).await?))
}

/// Derive an HKDF key from a secret and salt.
pub(crate) async fn hkdf_with_salt(secret_bytes: &[u8], salt: &[u8], info: &str) -> JsResult<CryptoKey> {
    let mut expanded_key = [0; AES_KEY_SIZE];

    let crypto = subtle_crypto()?;
    
    // Import 
    let secret_bytes = Uint8Array::from(secret_bytes);
    let hkdf_key = import_aes_key(
        &Object::from(secret_bytes), "HKDF", ["deriveBits"].as_ref()
    ).await?;


    // Derive
    // let derive_params = HkdfParams::new("HKDF", "SHA-256", salt, info.as_bytes());
    let derive_params = HkdfParams::new(
        "HKDF",
        &JsValue::from_str("SHA-256"),
        &Uint8Array::from(salt),
        &Uint8Array::from(info.as_bytes()),
    );
    let derive_bits = crypto_method(
        crypto.derive_bits_with_object(
            &derive_params,
            &hkdf_key,
            AES_KEY_SIZE as u32 * 8,
        )
    ).await?;

    // Import Again
    let derive_bits = derive_bits.dyn_into::<Object>().expect("derive bits to be an object");
    let key = import_aes_key(
        &derive_bits, "AES-GCM", ["encrypt", "decrypt"].as_ref()
    ).await?;

    Ok(key)
}

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

/* Misc Utilities */

// pub(crate) fn generate_info(encryptor: &[u8], decryptor: &[u8]) -> String {
//     format!(
//         "use=key_seal,encryptor={},decryptor={}",
//         pretty_fingerprint(encryptor),
//         pretty_fingerprint(decryptor),
//     )
// }

// pub(crate) fn base64_decode(data: &str) -> JsResult<Vec<u8>> {
//     B64.decode(data).map_err(KeySealError::bad_base64)
// }

// pub(crate) fn base64_encode(data: &[u8]) -> String {
//     B64.encode(data)
// }

