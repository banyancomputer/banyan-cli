use async_trait::async_trait;

use crate::key_seal::common::*;
use crate::key_seal::generate_info;
use crate::key_seal::wasm::*;
use crate::key_seal::KeySealError;

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
        let ec_shared_secret =
            internal::ec_derive_shared_secret(&recipient_key.private_key, &ephemeral_public_key.0)
                .await
                .map_err(KeySealError::subtle_crypto_error)?;

        let info = generate_info(
            ephemeral_public_key.fingerprint().await?.as_ref(),
            recipient_key.fingerprint().await?.as_ref(),
        );

        let shared_key = internal::hkdf_derive_aes_key_with_salt(
            &ec_shared_secret,
            &self.salt,
            &info,
            "AES-KW",
            &["unwrapKey", "wrapKey"],
        )
        .await?;

        let wrapped_key: [u8; AES_KEY_SIZE + 8] = self.data;
        let unwrapped_key = internal::aes_unwrap_key(
            &wrapped_key,
            &shared_key,
            "AES-KW",
            &["wrapKey", "unwrapKey"],
        )
        .await?;

        let unwrapped_key_vec = internal::aes_export_key(&unwrapped_key).await?;

        Ok(SymmetricKey(
            unwrapped_key_vec
                .try_into()
                .expect("unwrapped key is always valid"),
        ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    pub async fn random_struct() -> Result<EncryptedSymmetricKey, KeySealError> {
        let mut data = [0u8; AES_KEY_SIZE + 8];
        internal::random_bytes(&mut data)?;
        let mut salt = [0u8; SALT_SIZE];
        internal::random_bytes(&mut salt)?;
        let mut public_key = [0u8; 65];
        internal::random_bytes(&mut public_key)?;

        let encrypted_key = EncryptedSymmetricKey {
            data,
            salt,
            public_key: public_key.to_vec(),
        };
        Ok(encrypted_key)
    }

    #[wasm_bindgen_test]
    async fn export_import() -> Result<(), KeySealError> {
        // Get random values
        let encrypted_key = random_struct().await?;
        let serialized = encrypted_key.export();
        let imported = EncryptedSymmetricKey::import(&serialized)?;

        assert_eq!(imported.data, encrypted_key.data);
        assert_eq!(imported.salt, encrypted_key.salt);
        assert_eq!(imported.public_key, encrypted_key.public_key.to_vec());
        Ok(())
    }
}
