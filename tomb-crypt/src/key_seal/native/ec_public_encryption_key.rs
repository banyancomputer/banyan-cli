use async_trait::async_trait;
use openssl::pkey::{PKey, Public};

use crate::key_seal::common::*;
use crate::key_seal::native::*;
use crate::key_seal::KeySealError;

pub struct EcPublicEncryptionKey(pub(crate) PKey<Public>);

#[async_trait(?Send)]
impl WrappingPublicKey for EcPublicEncryptionKey {
    type Error = KeySealError;

    async fn export(&self) -> Result<Vec<u8>, KeySealError> {
        self.0
            .public_key_to_pem()
            .map_err(KeySealError::export_failed)
    }

    async fn export_bytes(&self) -> Result<Vec<u8>, KeySealError> {
        self.0
            .public_key_to_der()
            .map_err(KeySealError::export_failed)
    }

    async fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError> {
        Ok(internal::fingerprint(&self.0))
    }

    async fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError> {
        let raw_public =
            PKey::public_key_from_pem(pem_bytes).expect("parsing a valid pem public key");
        Ok(Self(raw_public))
    }

    async fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError> {
        let raw_public =
            PKey::public_key_from_der(der_bytes).expect("parsing a valid der public key");
        Ok(Self(raw_public))
    }
}
