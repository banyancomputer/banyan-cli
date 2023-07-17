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
        // self.0
        //     .public_key_to_pem()
        //     .map_err(KeySealError::export_failed)
    }

    async fn export_bytes(&self) -> Result<Vec<u8>, KeySealError> {
        todo!()
        // self.0
        //     .public_key_to_der()
        //     .map_err(KeySealError::export_failed)
    }

    async fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError> {
        todo!()
        // Ok(internal::fingerprint(&self.0))
    }

    async fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError> {
        todo!()
        // let raw_public =
        //     PKey::public_key_from_pem(pem_bytes).expect("parsing a valid pem public key");
        // Ok(Self(raw_public))
    }

    async fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError> {
        todo!()
        // let raw_public =
        //     PKey::public_key_from_der(der_bytes).expect("parsing a valid der public key");
        // Ok(Self(raw_public))
    }
}
