use async_trait::async_trait;
use web_sys::CryptoKey;

use crate::key_seal::common::*;
use crate::key_seal::wasm::*;
use crate::key_seal::KeySealError;

pub struct EcEncryptionKey {
    pub(crate) private_key: CryptoKey,
    pub(crate) public_key: Option<CryptoKey>,
}

#[async_trait(?Send)]
impl WrappingPrivateKey for EcEncryptionKey {
    type Error = KeySealError;
    type PublicKey = EcPublicEncryptionKey;

    async fn export(&self) -> Result<Vec<u8>, KeySealError> {
        internal::export_ec_key_pem("pkcs8", &self.private_key)
            .await
            .map_err(KeySealError::export_failed)
    }

    async fn export_bytes(&self) -> Result<Vec<u8>, KeySealError> {
        internal::export_ec_key_der("pkcs8", &self.private_key)
            .await
            .map_err(KeySealError::export_failed)
    }

    async fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError> {
        self.public_key()?.fingerprint().await
    }

    async fn generate() -> Result<Self, KeySealError> {
        let key_pair = internal::generate_ec_encryption_key_pair()
            .await
            .map_err(KeySealError::subtle_crypto_error)?;
        let private_key = internal::private_key(&key_pair);
        let public_key = internal::public_key(&key_pair);
        Ok(Self {
            private_key,
            public_key: Some(public_key),
        })
    }

    async fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError> {
        let private_key = internal::import_ec_key_pem("pkcs8", pem_bytes)
            .await
            .map_err(KeySealError::bad_format)?;
        Ok(Self {
            private_key,
            public_key: None,
        })
    }

    async fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError> {
        let private_key = internal::import_ec_key_der("pkcs8", der_bytes)
            .await
            .map_err(KeySealError::bad_format)?;
        Ok(Self {
            private_key,
            public_key: None,
        })
    }

    fn public_key(&self) -> Result<EcPublicEncryptionKey, KeySealError> {
        let public_key = self
            .public_key
            .as_ref()
            .ok_or(KeySealError::public_key_unavailable())?;
        Ok(EcPublicEncryptionKey(public_key.clone()))
    }
}
