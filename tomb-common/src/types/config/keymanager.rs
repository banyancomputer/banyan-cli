use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error;
use wnfs::private::{AesKey, ExchangeKey, PrivateKey, RsaPrivateKey, RsaPublicKey, TemporalKey};

/// Simply a Map from RSA Public Key fingerprints to the encrypted Temporal Keys they created
#[derive(Serialize, Deserialize)]
pub struct KeyManager {
    // The unencrypted TemporalKey
    root: RefCell<TemporalKey>,
    // A map from RSA Public Key fingerprints to their encrypted Temporal Keys
    pub map: RefCell<HashMap<String, Vec<u8>>>,
}

impl Default for KeyManager {
    fn default() -> Self {
        let default_aes_bytes: [u8; 32] = [0; 32];
        Self {
            root: RefCell::new(TemporalKey(AesKey::new(default_aes_bytes))),
            map: RefCell::new(HashMap::new()),
        }
    }
}

impl KeyManager {
    pub async fn update_temporal_key(&self, temporal_key: &TemporalKey) -> Result<()> {
        // Update the TemporalKey
        *self.root.borrow_mut() = temporal_key.clone();
        // Now that the TemporalKey has changed, reencrypt using all the existing RsaPublicKeys
        let map = self.map.borrow().clone();
        // For each Public Key present in the map
        for (der, _) in map {
            // Reconstruct the PublicKey form the DER hex
            let public_key = RsaPublicKey::from_der(&hex::decode(&der)?)?;
            // Reencrypt the TemporalKey using this
            let new_encrypted_temporal_key = public_key.encrypt(temporal_key.0.as_bytes()).await?;
            // Insert the reencrypted version of the TemporalKey
            self.map
                .borrow_mut()
                .insert(der, new_encrypted_temporal_key);
        }

        Ok(())
    }

    pub async fn insert(&self, key: &RsaPublicKey) -> Result<()> {
        let root = self.root.borrow().clone();
        // Encrypt the bytes
        let encrypted_key = key.encrypt(root.0.as_bytes()).await?;
        // Grab the public
        let der = hex::encode(key.to_der()?);
        // Insert into the hashmap
        self.map.borrow_mut().insert(der, encrypted_key);
        Ok(())
    }

    pub async fn retrieve(&self, key: &RsaPrivateKey) -> Result<TemporalKey> {
        // Grab the fingerprint
        let der = hex::encode(key.get_public_key().to_der()?);
        // Grab the encrypted key associated with the fingerprint
        let map = self.map.borrow().clone();
        if let Some(encrypted_key) = map.get(&der) {
            // Decrypt
            let aes_buf = key.decrypt(encrypted_key).await?;
            // Create struct
            let temporal_key = TemporalKey(AesKey::new(aes_buf.as_slice().try_into()?));
            // Return
            Ok(temporal_key)
        } else {
            Err(KeyError::Missing.into())
        }
    }

    // pub async fn initialize_if_empty(&self) -> Result<()> {
    //     // If we've not stored any keys yet
    //     if self.map.borrow().len() == 0 {
    //         // Create PrivateKey
    //         let new_key = RsaPrivateKey::new()?;
    //         // Insert the newly generated PublicKey
    //         self.insert(&new_key.get_public_key()).await?;
    //         // Serialize the PrivateKey to disk

    //         Ok(())
    //     } else {
    //         Ok(())
    //     }
    // }
}

/// Key errors.
#[derive(Debug, Error)]
pub(crate) enum KeyError {
    /// Missing a key
    #[error("There is no key in this bucket matching your PrivateKey")]
    Missing,
}
