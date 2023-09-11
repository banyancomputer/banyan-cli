use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tomb_crypt::prelude::*;
use wnfs::private::{TemporalKey, AccessKey};

// TODO: This should probably just be an encrypted blob. Can we extend EcEncrytionKey to work for this?
// TODO: How can we use PrivateRefSerializable here?
/// Basically just a private Ref but with an EncryptedSymetric key
/// in place of an unencrypted TemporalKey
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedAccessKey {
    /// Encrypted Skip-ratchet-derived key (as a string). Gives read access to the revision pointed to and any newer revisions.
    #[serde(rename = "encryptedTemporalKey")]
    pub encrypted_temporal_key_string: String,
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
        // Represent the entire access key as bytes
        let all_bytes = Into::<Vec<u8>>::into(access_key);
        // Encrypt it using the temporal key
        let encrypted_access_key = temporal_key.key_wrap_encrypt(&all_bytes)?;


        // Wrap the temporal key in a Symmetric Key
        let symmetric_key: SymmetricKey = (*temporal_key.as_bytes()).into();
        // Encrypt the symmetric key for the recipient key
        let encrypted_temporal_key = symmetric_key
            .encrypt_for(recipient_key)
            .await
            .expect("could not encrypt symmetric key for recipient");
        let encrypted_temporal_key_string = encrypted_temporal_key.export();

        
        // Return the EncryptedPrivateRef
        Ok(Self {
            encrypted_temporal_key_string,
            encrypted_access_key
        })
    }

    /// Decrypt an EncryptedPrivateRef with a recipient key
    pub async fn decrypt_with(self, recipient_key: &EcEncryptionKey) -> Result<AccessKey> {
        // Check if this is an empty string
        if self.encrypted_temporal_key_string.is_empty() {
            return Err(anyhow!("encrypted temporal key string is empty"));
        }
        // Get the encrypted temporal key from the string
        let encrypted_temporal_key =
            EncryptedSymmetricKey::import(&self.encrypted_temporal_key_string)
                .expect("could not import encrypted temporal key");
        // Decrypt the encrypted temporal key with the recipient key
        let temporal_key = encrypted_temporal_key
            .decrypt_with(recipient_key)
            .await
            .expect("could not decrypt encrypted temporal key");
        let temporal_key_slice = temporal_key.as_ref();
        if temporal_key_slice.len() != 32 {
            return Err(anyhow!("temporal key is not 32 bytes"));
        }
        let temporal_key: TemporalKey = serde_json::from_slice(temporal_key_slice)?;
        let all_bytes = temporal_key.key_wrap_decrypt(&self.encrypted_access_key)?;
        Ok(Into::<AccessKey>::into(all_bytes.as_slice()))
    }
}
