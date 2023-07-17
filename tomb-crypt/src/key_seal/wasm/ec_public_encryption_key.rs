use async_trait::async_trait;

use web_sys::CryptoKey;

use crate::key_seal::KeySealError;
use crate::key_seal::common::*;
use crate::key_seal::wasm::*;

pub struct EcPublicEncryptionKey(pub(crate) CryptoKey);

#[async_trait(?Send)]
impl WrappingPublicKey for EcPublicEncryptionKey {
    type Error = KeySealError;

    async fn export(&self) -> Result<Vec<u8>, KeySealError> {
        todo!()
    }

    async fn export_bytes(&self) -> Result<Vec<u8>, KeySealError> {
        internal::export_ec_key_der("spki", &self.0).await.map_err(|err| KeySealError::subtle_crypto_error(err.into()))
    }

    async fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError> {
        Ok(internal::fingerprint_public_ec_key(&self.0).await.map_err(|err| KeySealError::subtle_crypto_error(err.into()))?)
    }

    async fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError> {
        todo!()
    }

    async fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError> {
        let public_key = internal::import_ec_key_der("spki", der_bytes).await.map_err(|err| KeySealError::subtle_crypto_error(err.into()))?;
        Ok(Self(public_key))
    }
}
