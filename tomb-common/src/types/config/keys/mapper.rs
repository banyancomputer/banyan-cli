use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wnfs::private::{AesKey, ExchangeKey, PrivateKey, TemporalKey};

use crate::crypto::rsa::{RsaPrivateKey, RsaPublicKey};

use super::error::KeyError;

#[derive(Default, Serialize, Deserialize, PartialEq)]
pub struct Mapper(HashMap<String, (Vec<u8>, Vec<u8>)>);

impl Mapper {
    pub async fn update_temporal_key(&mut self, new_key: &TemporalKey) -> Result<()> {
        // For each Public Key present in the map
        for (fingerprint, (der, _)) in self.0.clone() {
            let public_key = RsaPublicKey::from_der(&der)?;
            // Reencrypt the TemporalKey using this
            let new_encrypted_root_key = public_key.encrypt(new_key.0.as_bytes()).await?;
            // Insert the reencrypted version of the TemporalKey
            self.0.insert(fingerprint, (der, new_encrypted_root_key));
        }

        Ok(())
    }

    pub async fn insert_public_key(
        &mut self,
        temporal_key: &Option<TemporalKey>,
        new_key: &RsaPublicKey,
    ) -> Result<()> {
        // Grab the public key's fingerprint
        let fingerprint = hex::encode(new_key.get_fingerprint()?);
        // Represent the public key as DER bytes
        let der = new_key.to_der()?;

        // If there is a valid temporal key
        if let Some(temporal_key) = temporal_key {
            // Encrypt the bytes
            let encrypted_temoral_key = new_key.encrypt(temporal_key.0.as_bytes()).await?;
            // Insert into the hashmap, using fingerprint as key
            self.0.insert(fingerprint, (der, encrypted_temoral_key));
        }
        // If a valid key does not yet exist
        else {
            // Insert an empty array as the "encrypted" bytes
            self.0.insert(fingerprint, (der, vec![]));
        }

        // Return Ok
        Ok(())
    }

    pub async fn reconstruct(&self, private_key: &RsaPrivateKey) -> Result<TemporalKey> {
        // Grab the fingerprint
        let fingerprint = hex::encode(private_key.get_public_key().get_fingerprint()?);
        // Grab the encrypted key associated with the fingerprint
        if let Some((_, encrypted_temporal_key)) = self.0.get(&fingerprint) {
            // Decrypt
            let aes_buf = private_key.decrypt(encrypted_temporal_key).await?;
            // Create struct
            let temporal_key = TemporalKey(AesKey::new(aes_buf.as_slice().try_into()?));
            // Return
            Ok(temporal_key)
        } else {
            Err(KeyError::Missing.into())
        }
    }
}

impl std::fmt::Debug for Mapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Mapper").field(&self.0.keys()).finish()
    }
}
