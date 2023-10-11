use js_sys::{ArrayBuffer, Error as JsError, JsString, Promise, Reflect, Uint8Array};
use pem::{encode, Pem};
use tomb_crypt::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{CryptoKey, CryptoKeyPair, SubtleCrypto};

pub(crate) type JsResult<T> = Result<T, JsError>;

/// Get the subtle crypto object from the window
pub(crate) fn subtle_crypto() -> JsResult<SubtleCrypto> {
    Ok(gloo::utils::window().crypto()?.subtle())
}

/// Run an Async function that returns a promise. Return as a Vec<u8>
async fn crypto_method(method: Result<Promise, JsValue>) -> JsResult<JsValue> {
    Ok(JsFuture::from(method?)
        .await
        .expect("crytpo method promise to succeed"))
}

fn get_private_key(key_pair: &CryptoKeyPair) -> CryptoKey {
    Reflect::get(key_pair, &JsString::from("privateKey"))
        .unwrap()
        .into()
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum EcKeyType {
    Encryption,
    Signature,
}

pub(crate) enum EcKeyFormat {
    Pkcs8,
    Spki,
}

impl EcKeyFormat {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            EcKeyFormat::Pkcs8 => "pkcs8",
            EcKeyFormat::Spki => "spki",
        }
    }
}

// CryptoKeyPair -> EcEncryptionKey
// CryptoKeyPair -> EcSignatureKey
// CryptoKey -> PublicEncryptionKey

pub async fn key_pair_to_encryption_key(key_pair: &CryptoKeyPair) -> JsResult<EcEncryptionKey> {
    let pem_bytes = export_ec_key_pem(EcKeyFormat::Pkcs8, &get_private_key(key_pair)).await?;
    Ok(EcEncryptionKey::import(&pem_bytes)
        .await
        .expect("failed to import EcEncryptionKey"))
}

pub async fn key_pair_to_signature_key(key_pair: &CryptoKeyPair) -> JsResult<EcSignatureKey> {
    let pem_bytes = export_ec_key_pem(EcKeyFormat::Pkcs8, &get_private_key(key_pair)).await?;
    Ok(EcSignatureKey::import(&pem_bytes)
        .await
        .expect("failed to import EcSignatureKey"))
}

pub async fn key_to_public_encryption_key(key: &CryptoKey) -> JsResult<EcPublicEncryptionKey> {
    let pem_bytes = export_ec_key_pem(EcKeyFormat::Spki, key).await?;
    Ok(EcPublicEncryptionKey::import(&pem_bytes)
        .await
        .expect("failed to import EcPublicEncryptionKey"))
}

/// Export a CryptoKey as DER bytes
pub(crate) async fn export_ec_key_der(format: EcKeyFormat, key: &CryptoKey) -> JsResult<Vec<u8>> {
    let crypto = subtle_crypto()?;
    let export = crypto_method(crypto.export_key(format.as_str(), key)).await?;
    let export = export
        .dyn_into::<ArrayBuffer>()
        .expect("export result to be an array buffer");
    let export = Uint8Array::new(&export).to_vec();
    Ok(export)
}

/// Export an ec key as a Vec<u8> in pem format
pub(crate) async fn export_ec_key_pem(
    format: EcKeyFormat,
    public_key: &CryptoKey,
) -> JsResult<Vec<u8>> {
    let tag: &str = match format {
        EcKeyFormat::Pkcs8 => "PRIVATE KEY",
        EcKeyFormat::Spki => "PUBLIC KEY",
    };
    let key_contents = export_ec_key_der(format, public_key).await?;
    let pem = Pem::new(tag, key_contents);
    Ok(encode(&pem).as_bytes().to_vec())
}
