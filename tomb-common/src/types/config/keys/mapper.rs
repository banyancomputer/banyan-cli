use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};
use wnfs::private::{AesKey, ExchangeKey, PrivateKey, RsaPrivateKey, RsaPublicKey, TemporalKey};

use super::error::KeyError;

#[derive(Default, Serialize, Deserialize)]
pub struct Mapper(RefCell<HashMap<String, (Vec<u8>, Vec<u8>)>>);

impl Mapper {
    pub async fn update_temporal_key(&self, new_key: &TemporalKey) -> Result<()> {
        let map = self.0.borrow().clone();
        // For each Public Key present in the map
        for (fingerprint, (der, _)) in map {
            let public_key = RsaPublicKey::from_der(&der)?;
            // Reencrypt the TemporalKey using this
            let new_encrypted_root_key = public_key.encrypt(new_key.0.as_bytes()).await?;
            // Insert the reencrypted version of the TemporalKey
            self
                .0
                .borrow_mut()
                .insert(fingerprint, (der, new_encrypted_root_key));
        }

        Ok(())
    }

    pub async fn insert_public_key(&self, temporal_key: &TemporalKey, new_key: &RsaPublicKey) -> Result<()> {
        // Encrypt the bytes
        let encrypted_temoral_key = new_key.encrypt(temporal_key.0.as_bytes()).await?;
        // Grab the public key's fingerprint and der bytes
        let fingerprint = hex::encode(new_key.get_sha1_fingerprint()?);
        let der = new_key.to_der()?;
        // Insert into the hashmap
        self.0.borrow_mut().insert(fingerprint.clone(), (der, encrypted_temoral_key));
        // Return Ok
        Ok(())
    }

    pub async fn reconstruct(&self, private_key: &RsaPrivateKey) -> Result<TemporalKey> {
        // Grab the fingerprint
        let fingerprint = hex::encode(private_key.get_public_key().get_sha1_fingerprint()?);
        // Clone map to prevent usage in async calls
        let map = self.0.borrow().clone();
        // Grab the encrypted key associated with the fingerprint
        if let Some((_, encrypted_temporal_key)) = map.get(&fingerprint) {
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
