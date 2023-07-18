use async_trait::async_trait;

use crate::key_seal::common::*;
use crate::key_seal::generate_info;
use crate::key_seal::wasm::*;
use crate::key_seal::EcPublicEncryptionKey;
use crate::key_seal::EncryptedSymmetricKey;
use crate::key_seal::KeySealError;

pub struct SymmetricKey(pub(crate) [u8; AES_KEY_SIZE]);

#[async_trait(?Send)]
impl PlainKey for SymmetricKey {
    type Error = KeySealError;
    type ProtectedKey = EncryptedSymmetricKey;
    type WrappingPublicKey = EcPublicEncryptionKey;

    async fn encrypt_for(
        &self,
        recipient_key: &Self::WrappingPublicKey,
    ) -> Result<Self::ProtectedKey, KeySealError> {
        // Generate ephemeral key pair
        let ephemeral_key_pair = EcEncryptionKey::generate().await?;

        // Derive shared secret with the recipient's public key
        let private_key = ephemeral_key_pair.private_key.clone();
        let ec_shared_secret = internal::ec_derive_shared_secret(&private_key, &recipient_key.0)
            .await
            .map_err(KeySealError::subtle_crypto_error)?;

        // Derive shared key with the shared secret
        let info = generate_info(
            ephemeral_key_pair.fingerprint().await?.as_ref(),
            recipient_key.fingerprint().await?.as_ref(),
        );
        let (salt, shared_key) =
            internal::hkdf_derive_aes_key(&ec_shared_secret, &info, "AES-KW", &["wrapKey"])
                .await
                .map_err(KeySealError::subtle_crypto_error)?;

        // Import the symmetric key so we can wrap it
        let key = internal::aes_import_key(
            &self.0,
            // Note: Not sure if algorithm or uses are required for our purposes,
            // but the web crypto API requires them as arguments.
            "AES-KW",
            &["wrapKey", "unwrapKey"],
        )
        .await?;

        // Wrap the SymmetricKey with the shared key
        let encrypted_key = internal::aes_wrap_key(&key, &shared_key).await?;

        let exported_ephemeral_key = ephemeral_key_pair.public_key()?.export_bytes().await?;

        Ok(EncryptedSymmetricKey {
            data: encrypted_key,
            salt,
            public_key: exported_ephemeral_key,
        })
    }
}

impl AsRef<[u8]> for SymmetricKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; AES_KEY_SIZE]> for SymmetricKey {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn from() -> Result<(), KeySealError> {
        // Get random values
        let mut key = [0u8; AES_KEY_SIZE];
        internal::random_bytes(&mut key)?;
        let _key = SymmetricKey::from(key);
        Ok(())
    }

    #[wasm_bindgen_test]
    fn as_ref() -> Result<(), KeySealError> {
        // Get random values
        let mut key = [0u8; AES_KEY_SIZE];
        internal::random_bytes(&mut key)?;
        let key = SymmetricKey::from(key);
        let _key_ref: &[u8] = key.as_ref();
        assert_eq!(key.as_ref(), &key.0);
        Ok(())
    }

    #[wasm_bindgen_test]
    async fn encrypt_for() -> Result<(), KeySealError> {
        // Get random values
        let mut key = [0u8; AES_KEY_SIZE];
        internal::random_bytes(&mut key)?;
        let key = SymmetricKey::from(key);

        // Generate a key pair
        let key_pair = EcEncryptionKey::generate().await?;

        // Encrypt the key
        let _encrypted_key = key.encrypt_for(&key_pair.public_key()?).await?;

        Ok(())
    }
}
