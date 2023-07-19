use async_trait::async_trait;
use openssl::pkey::{PKey, Private};

use crate::key_seal::common::*;
use crate::key_seal::native::*;
use crate::key_seal::KeySealError;

pub struct EcEncryptionKey(pub(crate) PKey<Private>);

#[async_trait(?Send)]
impl WrappingPrivateKey for EcEncryptionKey {
    type Error = KeySealError;
    type PublicKey = EcPublicEncryptionKey;

    async fn export(&self) -> Result<Vec<u8>, KeySealError> {
        self.0
            .private_key_to_pem_pkcs8()
            .map_err(KeySealError::export_failed)
    }

    async fn export_bytes(&self) -> Result<Vec<u8>, KeySealError> {
        self.0
            .private_key_to_der()
            .map_err(KeySealError::export_failed)
    }

    async fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError> {
        self.public_key()?.fingerprint().await
    }

    async fn generate() -> Result<Self, KeySealError> {
        let key = tokio::task::spawn_blocking(internal::generate_ec_key)
            .await
            .map_err(KeySealError::background_generation_failed)?;
        Ok(Self(key))
    }

    async fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError> {
        let raw_private =
            PKey::private_key_from_pem(pem_bytes).map_err(KeySealError::bad_format)?;

        Ok(Self(raw_private))
    }

    async fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError> {
        let raw_private =
            PKey::private_key_from_der(der_bytes).expect("parsing a valid der private key");
        Ok(Self(raw_private))
    }

    fn public_key(&self) -> Result<EcPublicEncryptionKey, KeySealError> {
        let ec_public = internal::public_from_private(&self.0);
        Ok(EcPublicEncryptionKey(ec_public))
    }
}
