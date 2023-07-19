use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use gloo::utils::window;
use js_sys::{
    Array, ArrayBuffer, Error as JsError, JsString, Object, Promise, Reflect, Uint8Array,
};
use pem::{encode, parse, Pem};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CryptoKey, CryptoKeyPair, EcKeyGenParams, EcdhKeyDeriveParams, HkdfParams, SubtleCrypto,
};

use crate::key_seal::common::{AES_KEY_SIZE, ECDH_SECRET_BYTE_SIZE, FINGERPRINT_SIZE, SALT_SIZE};
use crate::key_seal::wasm::KeySealError;

/* Wasm Utilities */

#[wasm_bindgen(js_name = "setPanicHook")]
pub fn set_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

fn js_array(values: &[&str]) -> JsValue {
    return JsValue::from(
        values
            .iter()
            .map(|x| JsValue::from_str(x))
            .collect::<Array>(),
    );
}

pub(crate) type JsResult<T> = Result<T, JsError>;

/*  Crypto Utilities*/

/// Get the crypto object from the window
pub(crate) fn crypto() -> Result<web_sys::Crypto, KeySealError> {
    window()
        .crypto()
        .map_err(|err| KeySealError::crypto_unavailable(err.into()))
}

#[cfg(test)]
/// Fill a slice with random bytes
/// # Arguments
/// * `buffer` - The buffer to fill
pub(crate) fn random_bytes(buffer: &mut [u8]) -> JsResult<()> {
    let crypto = crypto()?;
    crypto.get_random_values_with_u8_array(buffer)?;
    Ok(())
}

/* Subtle Crypto Utilities */

/// Get the subtle crypto object from the window
pub(crate) fn subtle_crypto() -> Result<SubtleCrypto, KeySealError> {
    let crypto = crypto()?;
    Ok(crypto.subtle())
}

/// Run an Async function that returns a promise. Return as a Vec<u8>
async fn crypto_method(method: Result<Promise, JsValue>) -> JsResult<JsValue> {
    Ok(JsFuture::from(method?)
        .await
        .expect("crytpo method promise to succeed"))
}

fn assert_key_algorithm(key: &CryptoKey, algorithm: &str) -> JsResult<()> {
    let key_algorithm_value = Reflect::get(key, &JsString::from("algorithm"))?;
    let key_algorithm_object: Object = key_algorithm_value.dyn_into::<Object>()?;
    let key_algorithm_name_value = Reflect::get(&key_algorithm_object, &JsString::from("name"))?;
    let key_algorithm_name_string: JsString = key_algorithm_name_value.dyn_into::<JsString>()?;
    let key_algorithm_name = key_algorithm_name_string.as_string().unwrap();
    assert_eq!(key_algorithm_name, algorithm);
    Ok(())
}

/* Ec Key Utilities */

/// Get the private key from a key pair
pub(crate) fn private_key(key_pair: &CryptoKeyPair) -> CryptoKey {
    Reflect::get(key_pair, &JsString::from("privateKey"))
        .unwrap()
        .into()
}

/// Get the public key from a key pair
pub(crate) fn public_key(key_pair: &CryptoKeyPair) -> CryptoKey {
    Reflect::get(key_pair, &JsString::from("publicKey"))
        .unwrap()
        .into()
}

/// Generate a new EC encryption key pair
pub(crate) async fn generate_ec_encryption_key_pair() -> JsResult<CryptoKeyPair> {
    let params = EcKeyGenParams::new("ECDH", "P-384");
    let usages = js_array(&["deriveBits"]);
    let crypto = subtle_crypto()?;
    let key_pair = crypto_method(crypto.generate_key_with_object(&params, true, &usages)).await?;
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
    ))
    .await?;
    Ok(import.into())
}

/// Import an ec key from a &[u8] in pem format
pub(crate) async fn import_ec_key_pem(format: &str, public_key: &[u8]) -> JsResult<CryptoKey> {
    let pem = std::str::from_utf8(public_key).unwrap();
    let pem = parse(pem).unwrap();
    let contents = pem.contents();
    let import = import_ec_key_der(format, contents).await?;
    Ok(import)
}

/// Export an ec key as a Vec<u8> in der format
pub(crate) async fn export_ec_key_der(format: &str, public_key: &CryptoKey) -> JsResult<Vec<u8>> {
    let crypto = subtle_crypto()?;
    let export = crypto_method(crypto.export_key(format, public_key)).await?;
    let export = export
        .dyn_into::<ArrayBuffer>()
        .expect("export result to be an array buffer");
    let export = Uint8Array::new(&export).to_vec();
    Ok(export)
}

/// Export an ec key as a Vec<u8> in pem format
pub(crate) async fn export_ec_key_pem(format: &str, public_key: &CryptoKey) -> JsResult<Vec<u8>> {
    let tag: &str = match format {
        "pkcs8" => "PRIVATE KEY",
        "spki" => "PUBLIC KEY",
        _ => panic!("invalid format"),
    };
    let key_contents = export_ec_key_der(format, public_key).await?;
    let pem = Pem::new(tag, key_contents);
    Ok(encode(&pem).as_bytes().to_vec())
}

/// Fingerprint an ec public key
pub(crate) async fn fingerprint_public_ec_key(
    public_key: &CryptoKey,
) -> JsResult<[u8; FINGERPRINT_SIZE]> {
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
        crypto.digest_with_str_and_u8_array("SHA-1", compressed_point.as_mut_slice()),
    )
    .await?;

    let digest = digest
        .dyn_into::<ArrayBuffer>()
        .expect("digest result to be an array buffer");

    let digest = Uint8Array::new(&digest).to_vec();

    let mut fingerprint = [0u8; FINGERPRINT_SIZE];
    fingerprint.copy_from_slice(&digest);

    Ok(fingerprint)
}

pub(crate) async fn ec_derive_shared_secret(
    private_key: &CryptoKey,
    public_key: &CryptoKey,
) -> JsResult<ArrayBuffer> {
    let crypto = subtle_crypto()?;

    // Derive bits from the private and public keys
    let ecdh_derive_params = EcdhKeyDeriveParams::new("ECDH", public_key);
    let exchange_secret = crypto_method(crypto.derive_bits_with_object(
        &ecdh_derive_params,
        private_key,
        ECDH_SECRET_BYTE_SIZE as u32 * 8,
    ))
    .await?;

    // Convert the bits to a Uint8Array
    let exchange_secret = exchange_secret
        .dyn_into::<ArrayBuffer>()
        .expect("exchange secret to be an array buffer");

    // Assert the Array Buffer is the correct size
    assert_eq!(exchange_secret.byte_length(), ECDH_SECRET_BYTE_SIZE as u32);

    Ok(exchange_secret)
}

/* AES Key Utilities */

/// Import an AES key from a &[u8] in raw format
/// # Arguments
/// * `key_data` - The key data to import
/// * `name` - The name of the key algorithm
/// * `uses` - The uses of the key
/// # Returns
/// * `CryptoKey` - The imported key within a Result
pub(crate) async fn aes_import_key(
    key_data: &[u8],
    name: &str,
    uses: &[&str],
) -> JsResult<CryptoKey> {
    let crypto = subtle_crypto()?;
    let import = crypto_method(crypto.import_key_with_str(
        "raw",
        &Uint8Array::from(key_data),
        name,
        true,
        &js_array(uses),
    ))
    .await?;
    Ok(import.into())
}

/// Import an AES key from a &[u8] in raw format
/// # Arguments
/// * `key_data` - The key data to import
/// * `name` - The name of the key algorithm
/// * `uses` - The uses of the key
/// # Returns
/// * `CryptoKey` - The imported key within a Result
async fn aes_import_key_with_buffer(
    key_data: &ArrayBuffer,
    name: &str,
    uses: &[&str],
) -> JsResult<CryptoKey> {
    let crypto = subtle_crypto()?;
    let import =
        crypto_method(crypto.import_key_with_str("raw", key_data, name, true, &js_array(uses)))
            .await?;
    Ok(import.into())
}

/// Export an AES key as a Vec<u8> in raw format
/// # Arguments
/// * `key` - The key to export
/// # Returns
/// * `Vec<u8>` - The exported key within a Result
pub(crate) async fn aes_export_key(key: &CryptoKey) -> JsResult<Vec<u8>> {
    let crypto = subtle_crypto()?;
    let export = crypto_method(crypto.export_key("raw", key)).await?;
    let export = export
        .dyn_into::<ArrayBuffer>()
        .expect("export result to be an array buffer");
    let export = Uint8Array::new(&export).to_vec();
    Ok(export)
}

/// Wrap an AES key with a wrapping key with AES-KW
/// # Arguments
/// * `key` - The key to wrap. This must be an AES key
/// * `wrapping_key` - The wrapping key to use
/// # Returns an result containing an array buffer with the wrapped key
pub(crate) async fn aes_wrap_key(
    key: &CryptoKey,
    wrapping_key: &CryptoKey,
) -> JsResult<[u8; AES_KEY_SIZE + 8]> {
    // Assert that wrapping key is an AES-KW key
    assert_key_algorithm(wrapping_key, "AES-KW")?;

    let crypto = subtle_crypto()?;
    let wrapped_key_buffer =
        crypto_method(crypto.wrap_key_with_str("raw", key, wrapping_key, "AES-KW")).await?;
    let wrapped_key_buffer = wrapped_key_buffer
        .dyn_into::<ArrayBuffer>()
        .expect("wrapped key to be an array buffer");
    let wrapped_key = Uint8Array::new(&wrapped_key_buffer).to_vec();
    let wrapped_key: [u8; AES_KEY_SIZE + 8] = wrapped_key
        .as_slice()
        .try_into()
        .expect("wrapped key to be the correct size");

    Ok(wrapped_key)
}

/// Unwrap an AES key with a wrapping key with AES-KW
/// # Arguments
/// * `key_data` - The key data to import
/// * `wrapping_key` - The wrapping key to use
/// * `unwrapped_key_algorithm` - The algorithm of the unwrapped key
/// * `unwrapped_key_uses` - The uses of the unwrapped key
/// # Returns an result containing a CryptoKey with the unwrapped key
pub(crate) async fn aes_unwrap_key(
    key_data: &[u8],
    wrapping_key: &CryptoKey,
    unwrapped_key_algorithm: &str,
    unwrapped_key_uses: &[&str],
) -> JsResult<CryptoKey> {
    // Assert that wrapping key is an AES-KW key
    assert_key_algorithm(wrapping_key, "AES-KW")?;

    let crypto = subtle_crypto()?;
    let mut data = [0u8; AES_KEY_SIZE + 8];
    data.copy_from_slice(key_data);

    let unwrapped_key = crypto_method(crypto.unwrap_key_with_u8_array_and_str_and_str(
        "raw",
        data.as_mut(),
        wrapping_key,
        "AES-KW",
        unwrapped_key_algorithm,
        true,
        &js_array(unwrapped_key_uses),
    ))
    .await?;
    Ok(unwrapped_key.into())
}

/* HKDF Utilities */

/// Derive am HKDF key from a secret.
/// # Arguments
/// * `secret_bytes` - The secret to derive the key from
/// * `info` - The info to use in the HKDF derivation
/// * `algorithm` - The algorithm to use in the HKDF derivation
/// * `uses` - The uses to use in the HKDF derivation
/// # Returns
/// * `([u8; SALT_SIZE], CryptoKey)` - The salt and the derived key
pub(crate) async fn hkdf_derive_aes_key(
    secret_bytes: &ArrayBuffer,
    info: &str,
    algorithm: &str,
    uses: &[&str],
) -> JsResult<([u8; SALT_SIZE], CryptoKey)> {
    let mut salt = [0u8; SALT_SIZE];
    let crypto = crypto()?;
    crypto
        .get_random_values_with_u8_array(&mut salt)
        .expect("unable to generate random IV");
    Ok((
        salt,
        hkdf_derive_aes_key_with_salt(secret_bytes, &salt, info, algorithm, uses).await?,
    ))
}

/// Derive an HKDF key from a secret and salt.
/// # Arguments
/// * `secret_bytes` - The secret to derive the key from
/// * `salt` - The salt to use in the HKDF derivation
/// * `info` - The info to use in the HKDF derivation
/// * `algorithm` - The algorithm to use in the HKDF derivation
/// * `uses` - The uses to use in the HKDF derivation
/// # Returns
/// * `([u8; SALT_SIZE], CryptoKey)` - The salt and the derived key
pub(crate) async fn hkdf_derive_aes_key_with_salt(
    secret_bytes: &ArrayBuffer,
    salt: &[u8],
    info: &str,
    algorithm: &str,
    uses: &[&str],
) -> JsResult<CryptoKey> {
    let crypto = subtle_crypto()?;

    // Import the secret as an HKDF key
    let hkdf_key =
        aes_import_key_with_buffer(secret_bytes, "HKDF", ["deriveBits"].as_ref()).await?;

    // Derive bits from the HKDF key
    let derive_params = HkdfParams::new(
        "HKDF",
        &JsValue::from_str("SHA-256"),
        &Uint8Array::from(salt),
        &Uint8Array::from(info.as_bytes()),
    );
    let derive_bits = crypto_method(crypto.derive_bits_with_object(
        &derive_params,
        &hkdf_key,
        AES_KEY_SIZE as u32 * 8,
    ))
    .await?;

    // Import the bits as a raw AES key
    let derive_bits = derive_bits
        .dyn_into::<ArrayBuffer>()
        .expect("derive bits to be an array buffer");
    let key = aes_import_key_with_buffer(&derive_bits, algorithm, uses).await?;

    // Return the key
    Ok(key)
}

/* Misc Utilities */

pub(crate) fn base64_decode(data: &str) -> Result<Vec<u8>, KeySealError> {
    B64.decode(data).map_err(KeySealError::bad_base64)
}

pub(crate) fn base64_encode(data: &[u8]) -> String {
    B64.encode(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn aes_wrap_unwrap() -> JsResult<()> {
        let mut key_bytes = [0u8; AES_KEY_SIZE];
        let mut wrapping_key_bytes = [0u8; AES_KEY_SIZE];

        random_bytes(&mut key_bytes)?;
        random_bytes(&mut wrapping_key_bytes)?;

        // Generate two random AES-KW keys
        let key = aes_import_key(&key_bytes, "AES-KW", &["wrapKey", "unwrapKey"]).await?;
        let wrapping_key =
            aes_import_key(&wrapping_key_bytes, "AES-KW", &["wrapKey", "unwrapKey"]).await?;

        // Wrap the key
        let wrapped_key: [u8; AES_KEY_SIZE + 8] = aes_wrap_key(&key, &wrapping_key).await?;
        let unwrapped_key = aes_unwrap_key(
            &wrapped_key,
            &wrapping_key,
            "AES-KW",
            &["wrapKey", "unwrapKey"],
        )
        .await?;

        // Assert the unwrapped key is the same as the original key
        let key_bytes = aes_export_key(&key).await?;
        let unwrapped_key_bytes = aes_export_key(&unwrapped_key).await?;

        assert_eq!(key_bytes, unwrapped_key_bytes);

        Ok(())
    }
}
