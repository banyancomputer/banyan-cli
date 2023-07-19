use async_trait::async_trait;

use crate::key_seal::common::*;
use crate::key_seal::native::*;
use crate::key_seal::{generate_info, KeySealError};

pub struct EncryptedSymmetricKey {
    pub(crate) data: [u8; AES_KEY_SIZE + 8],
    pub(crate) salt: [u8; SALT_SIZE],
    pub(crate) public_key: Vec<u8>,
}

#[async_trait(?Send)]
impl ProtectedKey for EncryptedSymmetricKey {
    type Error = KeySealError;
    type PlainKey = SymmetricKey;
    type WrappingPrivateKey = EcEncryptionKey;

    async fn decrypt_with(
        &self,
        recipient_key: &EcEncryptionKey,
    ) -> Result<SymmetricKey, KeySealError> {
        let ephemeral_public_key =
            EcPublicEncryptionKey::import_bytes(self.public_key.as_ref()).await?;
        let ecdh_shared_secret =
            internal::ecdh_exchange(&recipient_key.0, &ephemeral_public_key.0)?;

        let info = generate_info(
            ephemeral_public_key.fingerprint().await?.as_ref(),
            recipient_key.fingerprint().await?.as_ref(),
        );
        let hkdf_shared_secret =
            internal::hkdf_with_salt(&ecdh_shared_secret, self.salt.as_ref(), &info);

        let temporal_key_bytes = internal::unwrap_key(&hkdf_shared_secret, self.data.as_ref());

        Ok(SymmetricKey(temporal_key_bytes))
    }

    fn export(&self) -> String {
        [
            internal::base64_encode(&self.salt),
            internal::base64_encode(&self.data),
            internal::base64_encode(self.public_key.as_ref()),
        ]
        .join(".")
    }

    fn import(serialized: &str) -> Result<Self, KeySealError> {
        let components: Vec<_> = serialized.split('.').collect();

        let raw_salt = internal::base64_decode(components[0])?;
        let mut salt = [0u8; SALT_SIZE];
        salt.copy_from_slice(raw_salt.as_ref());

        let raw_data = internal::base64_decode(components[1])?;
        let mut data = [0u8; AES_KEY_SIZE + 8];
        data.copy_from_slice(raw_data.as_ref());

        let public_key = internal::base64_decode(components[2])?;

        Ok(Self {
            salt,
            data,
            public_key,
        })
    }
}
