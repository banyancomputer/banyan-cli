use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};
use thiserror::Error;
use wnfs::private::{AesKey, ExchangeKey, PrivateKey, RsaPrivateKey, RsaPublicKey, TemporalKey};

/// Simply a Map from RSA Public Key fingerprints to the encrypted Temporal Keys they created
pub struct KeyManager {
    // The unencrypted TemporalKey
    root: RefCell<TemporalKey>,
    original: RefCell<TemporalKey>,
    // A map from RSA Public Key fingerprints to their encrypted Temporal Keys
    pub root_map: KeyMapper,
    pub original_map: KeyMapper,
}

#[derive(Default, Serialize, Deserialize)]
pub struct KeyMapper(RefCell<HashMap<String, (Vec<u8>, Vec<u8>)>>);

impl KeyMapper {
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

impl KeyManager {
    pub async fn update_temporal_key(&self, new_key: &TemporalKey) -> Result<()> {
        // Update the TemporalKey
        *self.root.borrow_mut() = new_key.clone();
        self.root_map.update_temporal_key(new_key).await?;
        Ok(())
    }

    pub async fn set_original_key(&self, new_key: &TemporalKey) -> Result<()> {
        // Update the TemporalKey
        *self.original.borrow_mut() = new_key.clone();
        self.original_map.update_temporal_key(new_key).await?;
        Ok(())
    }

    pub async fn insert(&self, new_key: &RsaPublicKey) -> Result<()> {
        let root = self.root.borrow().clone();
        let original = self.original.borrow().clone();
        
        self.root_map.insert_public_key(&root, new_key).await?;
        self.original_map.insert_public_key(&original, new_key).await?;

        Ok(())
    }

    pub async fn retrieve_current(&self, private_key: &RsaPrivateKey) -> Result<TemporalKey> {
        self.root_map.reconstruct(&private_key).await
    }

    pub async fn retrieve_original(&self, private_key: &RsaPrivateKey) -> Result<TemporalKey> {
        self.original_map.reconstruct(&private_key).await
    }
}

impl Default for KeyManager {
    fn default() -> Self {
        let default_aes_bytes: [u8; 32] = [0; 32];
        Self {
            root: RefCell::new(TemporalKey(AesKey::new(default_aes_bytes))),
            original: RefCell::new(TemporalKey(AesKey::new(default_aes_bytes))),
            root_map: Default::default(),
            original_map: Default::default(),
        }
    }
}

impl Serialize for KeyManager {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.root_map, &self.original_map).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for KeyManager {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (root_map, original_map) = <(KeyMapper, KeyMapper)>::deserialize(deserializer)?;
        let default = Self::default();
        Ok(Self {
            root: default.root,
            original: default.original,
            root_map,
            original_map
        })
    }
}

/// Key errors.
#[derive(Debug, Error)]
pub(crate) enum KeyError {
    /// Missing a key
    #[error("There is no key in this bucket matching your PrivateKey")]
    Missing,
}
