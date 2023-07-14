use super::mapper::Mapper;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tomb_crypt::prelude::{EcEncryptionKey, EcPublicEncryptionKey};
use wnfs::private::TemporalKey;

/// Simply a Map from RSA Public Key fingerprints to the encrypted Temporal Keys they created
#[derive(Debug, Default, PartialEq)]
pub struct Manager {
    /// The unencrypted original TemporalKey
    original: Option<TemporalKey>,
    /// The unencrypted current TemporalKey
    current: Option<TemporalKey>,
    /// Mapper for original TemporalKey
    pub original_map: Mapper,
    /// Mapper for current TemporalKey
    pub current_map: Mapper,
}

impl Manager {
    /// Update the current TemporalKey
    pub fn update_current_key(&mut self, new_key: &TemporalKey) -> Result<()> {
        // Update the TemporalKey
        self.current = Some(new_key.clone());
        self.current_map.update_temporal_key(new_key)?;
        Ok(())
    }

    /// Update the original TemporalKey
    pub fn set_original_key(&mut self, new_key: &TemporalKey) -> Result<()> {
        // Update the TemporalKey
        self.original = Some(new_key.clone());
        self.original_map.update_temporal_key(new_key)?;
        Ok(())
    }

    /// Insert a new RsaPublicKey into both Mappers
    pub fn insert(&mut self, new_key: &EcPublicEncryptionKey) -> Result<()> {
        // Insert into original
        self.original_map
            .insert_public_key(&self.original, new_key)?;

        // Insert into current
        self.current_map.insert_public_key(&self.current, new_key)
    }

    /// Retrieve the current TemporalKey using a PrivateKey
    pub fn retrieve_current(&self, private_key: &EcEncryptionKey) -> Result<TemporalKey> {
        self.current_map.reconstruct(private_key)
    }

    /// Retrieve the original TemporalKey using a PrivateKey
    pub fn retrieve_original(&self, private_key: &EcEncryptionKey) -> Result<TemporalKey> {
        self.original_map.reconstruct(private_key)
    }

    /// Reload both keys into memory
    pub fn load_temporal_keys(&mut self, private_key: &EcEncryptionKey) -> Result<()> {
        self.current = Some(self.retrieve_current(private_key)?);
        self.original = Some(self.retrieve_original(private_key)?);
        Ok(())
    }
}

impl Serialize for Manager {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.current_map, &self.original_map).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Manager {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (current_map, original_map) = <(Mapper, Mapper)>::deserialize(deserializer)?;
        Ok(Self {
            original: None,
            current: None,
            original_map,
            current_map,
        })
    }
}
