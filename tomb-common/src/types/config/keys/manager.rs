use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap};

use wnfs::private::{AesKey, ExchangeKey, PrivateKey, RsaPrivateKey, RsaPublicKey, TemporalKey};

use super::mapper::Mapper;


/// Simply a Map from RSA Public Key fingerprints to the encrypted Temporal Keys they created
pub struct Manager {
    // The unencrypted TemporalKey
    root: RefCell<TemporalKey>,
    original: RefCell<TemporalKey>,
    // A map from RSA Public Key fingerprints to their encrypted Temporal Keys
    pub root_map: Mapper,
    pub original_map: Mapper,
}

impl Manager {
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

impl Default for Manager {
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

impl Serialize for Manager {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.root_map, &self.original_map).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Manager {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (root_map, original_map) = <(Mapper, Mapper)>::deserialize(deserializer)?;
        let default = Self::default();
        Ok(Self {
            root: default.root,
            original: default.original,
            root_map,
            original_map
        })
    }
}
