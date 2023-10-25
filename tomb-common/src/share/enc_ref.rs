use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use tomb_crypt::prelude::*;
use wnfs::{
    common::HashOutput,
    libipld::Cid,
    private::{PrivateRef, TemporalKey},
};

// TODO: This should probably just be an encrypted blob. Can we extend EcEncrytionKey to work for this?
// TODO: How can we use PrivateRefSerializable here?
/// Basically just a private Ref but with an EncryptedSymetric key
/// in place of an unencrypted TemporalKey
#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedPrivateRef {
    /// Sha3-256 hash of saturated namefilter. Used as the label for identifying revisions of PrivateNodes in the PrivateForest.
    #[serde(rename = "name")]
    pub saturated_name_hash: HashOutput,
    /// Encrypted Skip-ratchet-derived key (as a string). Gives read access to the revision pointed to and any newer revisions.
    #[serde(rename = "encryptedTemporalKey")]
    pub encrypted_temporal_key_string: String,
    /// CID that identifies the exact value in the multivalue.
    #[serde(rename = "contentCid")]
    pub content_cid: Cid,
}

impl EncryptedPrivateRef {
    /// Share a private Ref with a recipient key
    pub async fn encrypt_for(
        private_ref: &PrivateRef,
        recipient_key: &EcPublicEncryptionKey,
    ) -> Result<Self> {
        // Restore the temporal key from the Reference
        let temporal_key = private_ref.temporal_key.clone();
        // Wrap the temporal key in a Symmetric Key
        let temporal_key = SymmetricKey::from(temporal_key.0.bytes());
        // Encrypt the symmetric key for the recipient key
        let encrypted_temporal_key = temporal_key
            .encrypt_for(recipient_key)
            .await
            .expect("could not encrypt symmetric key for recipient");
        let encrypted_temporal_key_string = encrypted_temporal_key.export();
        // Return the EncryptedPrivateRef
        Ok(Self {
            saturated_name_hash: private_ref.saturated_name_hash,
            encrypted_temporal_key_string,
            content_cid: private_ref.content_cid,
        })
    }

    /// Decrypt an EncryptedPrivateRef with a recipient key
    pub async fn decrypt_with(self, recipient_key: &EcEncryptionKey) -> Result<PrivateRef> {
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
        let mut temporal_key = [0u8; 32];
        temporal_key.copy_from_slice(temporal_key_slice);
        let temporal_key = TemporalKey::from(temporal_key);
        // Return the PrivateRef
        Ok(PrivateRef {
            saturated_name_hash: self.saturated_name_hash,
            temporal_key,
            content_cid: self.content_cid,
        })
    }
}
