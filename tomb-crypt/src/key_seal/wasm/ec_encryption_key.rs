use async_trait::async_trait;
use js_sys::{
    ArrayBuffer, Uint8Array
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    CryptoKeyPair, SubtleCrypto, CryptoKey, EcKeyGenParams
};

use crate::key_seal::KeySealError;
use crate::key_seal::common::*;
use crate::key_seal::wasm::*;

pub struct EcEncryptionKey {
    pub(crate) private_key: CryptoKey,
    pub(crate) public_key: Option<CryptoKey>
}

#[async_trait(?Send)]
impl WrappingPrivateKey for EcEncryptionKey {
    type Error = KeySealError;
    type PublicKey = EcPublicEncryptionKey;

    async fn export(&self) -> Result<Vec<u8>, KeySealError> {
        todo!();
    }
    
    async fn export_bytes(&self) -> Result<Vec<u8>, KeySealError> {
        internal::export_ec_private_key(&self.private_key).await.map_err(|err| KeySealError::subtle_crypto_error(err.into()))
    }

    async fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError> {
        self.public_key()?.fingerprint().await
    }

    async fn generate() -> Result<Self, KeySealError> {
        // Ok(Self(internal::generate_ec_encryption_key_pair().await.map_err(|err| KeySealError::subtle_crypto_error(err.into()))?))
        let key_pair = internal::generate_ec_encryption_key_pair().await.map_err(|err| KeySealError::subtle_crypto_error(err.into()))?;
        let private_key = internal::private_key(&key_pair);
        let public_key = internal::public_key(&key_pair);
        Ok(Self { private_key, public_key: Some(public_key) })
    }

    async fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError> {
        todo!()
    }

    async fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError> {
        let private_key = internal::import_ec_private_key(der_bytes).await.map_err(|err| KeySealError::subtle_crypto_error(err.into()))?;
        Ok(Self { private_key, public_key: None })
    }

    fn public_key(&self) -> Result<EcPublicEncryptionKey, KeySealError> {
        let public_key = self.public_key.as_ref().ok_or(KeySealError::public_key_unavailable())?; 
        Ok(EcPublicEncryptionKey(public_key.clone()))
    }
}
