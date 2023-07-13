use super::error::KeyError;
use crate::crypto::rsa::{RsaPrivateKey, RsaPublicKey};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use wnfs::{
    libipld::Ipld,
    private::{AesKey, ExchangeKey, PrivateKey, TemporalKey},
};

#[derive(Default, PartialEq, Debug)]
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
        let fingerprint = new_key.get_fingerprint()?;
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
        let fingerprint = private_key.get_public_key().get_fingerprint()?;
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

impl Mapper {
    fn to_ipld(&self) -> Ipld {
        // New Map
        let mut map = BTreeMap::<String, Ipld>::new();
        // For each key value pair in the struct
        for (fingerprint, (public_key, encrypted_key)) in self.0.clone() {
            let mut sub_map = BTreeMap::<String, Ipld>::new();
            // Overwrite with fake data if there is no encrypted key
            let encrypted_key = if encrypted_key.len() == 384 {
                encrypted_key
            } else {
                [0; 384].to_vec()
            };
            // Insert the fingerprint
            sub_map.insert("public_key".to_string(), Ipld::Bytes(public_key));
            // Insert the encrypted key
            sub_map.insert("encrypted_key".to_string(), Ipld::Bytes(encrypted_key));
            // Insert the sub_map
            map.insert(fingerprint, Ipld::Map(sub_map));
        }
        // Return
        Ipld::Map(map)
    }

    fn from_ipld(ipld: Ipld) -> Result<Self> {
        // New Mapper
        let mut mapper = Mapper::default();

        // If we can get the Map
        if let Ipld::Map(map) = ipld {
            // For each key value pair in the IPLD
            for (fingerprint, ipld) in map {
                if let Ipld::Map(sub_map) = ipld &&
                    let Some(Ipld::Bytes(public_key)) = sub_map.get("public_key") &&
                    let Some(Ipld::Bytes(encrypted_key)) = sub_map.get("encrypted_key") {
                    // Use empty array if it is actually blank
                    let encrypted_key = if encrypted_key == &[0u8; 384] { vec![] } else { encrypted_key.to_vec() };
                    // Insert the new value into the mapper
                    mapper.0.insert(fingerprint, (public_key.to_vec(), encrypted_key));
                }
                else {
                    return Err(KeyError::Missing.into());
                }
            }
        }

        Ok(mapper)
    }
}

impl Serialize for Mapper {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_ipld().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Mapper {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ipld = Ipld::deserialize(deserializer)?;
        Ok(Self::from_ipld(ipld).unwrap())
    }
}

// impl std::fmt::Debug for Mapper {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_tuple("Mapper").field(&self.0.keys()).finish()
//     }
// }

#[cfg(test)]
mod test {
    use super::Mapper;
    use crate::crypto::rsa::RsaPrivateKey;
    use anyhow::Result;
    use wnfs::{
        common::dagcbor,
        private::{AesKey, TemporalKey},
    };

    #[tokio::test]
    async fn to_from_ipld() -> Result<()> {
        // Create new mapper
        let mut mapper1 = Mapper::default();
        // Wrapping Key
        let wrapping_key = RsaPrivateKey::default();
        // Public Key
        let public_key = wrapping_key.get_public_key();
        // Insert a public key
        mapper1.insert_public_key(&None, &public_key).await?;

        let mapper1_ipld = mapper1.to_ipld();
        let mut mapper2 = Mapper::from_ipld(mapper1_ipld)?;
        // Assert reconstruction
        assert_eq!(mapper1, mapper2);
        let temporal_key = TemporalKey(AesKey::new([7u8; 32]));
        // Update temporal key
        mapper2.update_temporal_key(&temporal_key).await?;

        let mapper2_ipld = mapper2.to_ipld();
        let mapper3 = Mapper::from_ipld(mapper2_ipld)?;
        // Assert reconstruction
        assert_eq!(mapper2, mapper3);

        // Assert decryption
        let new_temporal_key = mapper3.reconstruct(&wrapping_key).await?;
        assert_eq!(temporal_key, new_temporal_key);

        Ok(())
    }

    #[tokio::test]
    async fn serial_size() -> Result<()> {
        // Create new mapper
        let mut mapper1 = Mapper::default();
        // Wrapping Key
        let wrapping_key = RsaPrivateKey::default();
        // Public Key
        let public_key = wrapping_key.get_public_key();
        // Insert a public key
        mapper1.insert_public_key(&None, &public_key).await?;

        // Serialize
        let mapper1_bytes = dagcbor::encode(&mapper1)?;
        let mut mapper2: Mapper = dagcbor::decode(mapper1_bytes.as_slice())?;
        // Assert reconstruction
        assert_eq!(mapper1, mapper2);

        let temporal_key = TemporalKey(AesKey::new([7u8; 32]));
        // Update temporal key
        mapper2.update_temporal_key(&temporal_key).await?;

        let mapper2_bytes = dagcbor::encode(&mapper2)?;
        let mapper3: Mapper = dagcbor::decode(mapper2_bytes.as_slice())?;
        // Assert reconstruction
        assert_eq!(mapper2, mapper3);

        // Assert that updating the temporal key did not alter the size of the struct
        assert_eq!(mapper1_bytes.len(), mapper2_bytes.len());

        // Assert decryption
        let new_temporal_key = mapper3.reconstruct(&wrapping_key).await?;
        assert_eq!(temporal_key, new_temporal_key);

        Ok(())
    }
}
