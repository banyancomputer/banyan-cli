use aes_kw::KekAes256;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tomb_crypt::prelude::*;
use wnfs::private::AccessKey;

// TODO: This should probably just be an encrypted blob. Can we extend EcEncrytionKey to work for this?
// TODO: How can we use PrivateRefSerializable here?
/// Basically just a private Ref but with an EncryptedSymetric key
/// in place of an unencrypted TemporalKey
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedAccessKey {
    /// Encrypted Skip-ratchet-derived key (as a string). Gives read access to the revision pointed to and any newer revisions.
    #[serde(rename = "encryptedTemporalKey")]
    pub encrypted_aes_key_string: String,
    /// Encrypted Skip-ratchet-derived key (as a string). Gives read access to the revision pointed to and any newer revisions.
    #[serde(rename = "encryptedAccessKey")]
    pub encrypted_access_key: Vec<u8>,
}

impl EncryptedAccessKey {
    /// Share a private Ref with a recipient key
    pub async fn encrypt_for(
        access_key: &AccessKey,
        recipient_key: &EcPublicEncryptionKey,
    ) -> Result<Self> {
        // Extract the temporal key from the Reference
        let temporal_key = access_key.get_temporal_key()?;
        // Aes KeyWrapper
        let aes_key = <KekAes256>::from(*temporal_key.as_bytes());
        // Represent the entire access key as bytes
        let all_access_key_bytes = <Vec<u8>>::from(access_key);
        // Wrap with padding
        let encrypted_access_key = aes_key
            .wrap_with_padding_vec(&all_access_key_bytes)
            .expect("unable to wrap access key");
        // Wrap the temporal key in a Symmetric Key
        let symmetric_aes_key = SymmetricKey::from(*temporal_key.as_bytes());
        // Encrypt the symmetric key for the recipient key
        let encrypted_aes_key = symmetric_aes_key
            .encrypt_for(recipient_key)
            .await
            .expect("could not encrypt symmetric key for recipient");
        let encrypted_aes_key_string = encrypted_aes_key.export();

        // Return the EncryptedPrivateRef
        Ok(Self {
            encrypted_aes_key_string,
            encrypted_access_key,
        })
    }

    /// Decrypt an EncryptedPrivateRef with a recipient key
    pub async fn decrypt_with(self, recipient_key: &EcEncryptionKey) -> Result<AccessKey> {
        // Check if this is an empty string
        if self.encrypted_aes_key_string.is_empty() {
            return Err(anyhow!("encrypted temporal key string is empty"));
        }
        // Get the encrypted temporal key from the string
        let encrypted_aes_key = EncryptedSymmetricKey::import(&self.encrypted_aes_key_string)
            .expect("could not import encrypted temporal key");
        // Decrypt the encrypted temporal key with the recipient key
        let temporal_key = encrypted_aes_key
            .decrypt_with(recipient_key)
            .await
            .expect("could not decrypt encrypted temporal key");
        // Temporal Key
        let temporal_key_slice: &[u8; 32] = temporal_key.as_ref().try_into()?;
        // Aes Key
        let aes_key = <KekAes256>::from(*temporal_key_slice);
        // Unwrap the access key
        let all_access_key_bytes = aes_key
            .unwrap_with_padding_vec(&self.encrypted_access_key)
            .expect("unable to unwrap access key");
        // Into AccessKey
        Ok(Into::<AccessKey>::into(all_access_key_bytes.as_slice()))
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use tomb_crypt::prelude::{EcEncryptionKey, PrivateKey};

    use crate::{share::enc_key::EncryptedAccessKey, utils::tests::setup_key_test};

    #[tokio::test]
    async fn encrypt_decrypt() -> Result<()> {
        let access_key = setup_key_test("encrypt_decrypt").await?;
        let private_key = EcEncryptionKey::generate().await?;
        let public_key = private_key.public_key()?;
        let encrypted = EncryptedAccessKey::encrypt_for(&access_key, &public_key).await?;
        let decrypted = encrypted.decrypt_with(&private_key).await?;
        assert_eq!(access_key, decrypted);
        Ok(())
    }
}
