use async_trait::async_trait;
use web_sys::CryptoKey;

use crate::key_seal::common::*;
use crate::key_seal::wasm::*;
use crate::key_seal::KeySealError;

pub struct EcPublicEncryptionKey(pub(crate) CryptoKey);

#[async_trait(?Send)]
impl WrappingPublicKey for EcPublicEncryptionKey {
    type Error = KeySealError;

    async fn export(&self) -> Result<Vec<u8>, KeySealError> {
        internal::export_ec_key_pem("spki", &self.0)
            .await
            .map_err(KeySealError::subtle_crypto_error)
    }

    async fn export_bytes(&self) -> Result<Vec<u8>, KeySealError> {
        internal::export_ec_key_der("spki", &self.0)
            .await
            .map_err(KeySealError::subtle_crypto_error)
    }

    async fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError> {
        Ok(internal::fingerprint_public_ec_key(&self.0)
            .await
            .map_err(KeySealError::subtle_crypto_error)?)
    }

    async fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError> {
        let public_key = internal::import_ec_key_pem("spki", pem_bytes)
            .await
            .map_err(KeySealError::subtle_crypto_error)?;
        Ok(Self(public_key))
    }

    async fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError> {
        let public_key = internal::import_ec_key_der("spki", der_bytes)
            .await
            .map_err(KeySealError::subtle_crypto_error)?;
        Ok(Self(public_key))
    }
}
