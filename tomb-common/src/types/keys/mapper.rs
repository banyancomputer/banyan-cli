use super::error::KeyError;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tomb_crypt::prelude::{
    EcEncryptionKey, EcPublicEncryptionKey, EncryptedSymmetricKey, PlainKey, ProtectedKey,
    SymmetricKey, WrappingPrivateKey, WrappingPublicKey,
};
use tomb_crypt::pretty_fingerprint;

use std::collections::{BTreeMap, HashMap};
use wnfs::{
    libipld::Ipld,
    private::{AesKey, TemporalKey},
};

#[derive(Default, PartialEq)]
/// Map key fingerprints to PublicKeys and encrypted TemporalKeys
pub struct Mapper(HashMap<String, (Vec<u8>, String)>);

impl Mapper {
    /// Using each PublicKey to encrypt the new TemporalKey
    pub async fn update_temporal_key(&mut self, new_key: &TemporalKey) -> Result<()> {
        // Represent the TemporalKey as a SymmetricKey
        let symmetric = SymmetricKey::from(new_key.0.clone().bytes());

        // For each Public Key present in the map
        for (fingerprint, (der, _)) in self.0.clone() {
            let public_key = EcPublicEncryptionKey::import_bytes(&der)
                .await
                .map_err(|_| anyhow::anyhow!("crypt error!"))?;
            // The encrypted TemporalKey
            let protected_key = symmetric
                .encrypt_for(&public_key)
                .await
                .map_err(|_| anyhow::anyhow!("crypt error!"))?;
            // Insert the reencrypted version of the TemporalKey
            self.0.insert(fingerprint, (der, protected_key.export()));
        }

        Ok(())
    }

    /// Add a new PublicKey, save an encrypted TemporalKey if one was provided
    pub async fn insert_public_key(
        &mut self,
        temporal_key: &Option<TemporalKey>,
        new_key: &EcPublicEncryptionKey,
    ) -> Result<()> {
        // Grab the public key's fingerprint
        let fingerprint = pretty_fingerprint(
            &new_key
                .fingerprint()
                .await
                .map_err(|_| anyhow::anyhow!("crypt error!"))?,
        );
        // Represent the public key as DER bytes
        let der = new_key
            .export_bytes()
            .await
            .map_err(|_| anyhow::anyhow!("crypt error!"))?;

        // If there is a valid temporal key
        if let Some(temporal_key) = temporal_key {
            // Represent the TemporalKey as a SymmetricKey
            let symmetric = SymmetricKey::from(temporal_key.0.clone().bytes());
            // Encrypt the bytes
            let protected_key = symmetric
                .encrypt_for(new_key)
                .await
                .map_err(|_| anyhow::anyhow!("crypt error!"))?;
            // Insert into the hashmap, using fingerprint as key
            self.0.insert(fingerprint, (der, protected_key.export()));
        }
        // If a valid key does not yet exist
        else {
            // Insert an empty array as the "encrypted" bytes
            self.0.insert(fingerprint, (der, String::new()));
        }

        // Return Ok
        Ok(())
    }

    /// Decrypt the TemporalKey using a PrivateKey
    pub async fn reconstruct(&self, private_key: &EcEncryptionKey) -> Result<TemporalKey> {
        // Grab the fingerprint
        let fingerprint = pretty_fingerprint(
            &private_key
                .fingerprint()
                .await
                .map_err(|_| anyhow::anyhow!("crypt error!"))?,
        );

        // Grab the encrypted key associated with the fingerprint
        if let Some((_, protected_key_string)) = self.0.get(&fingerprint) {
            // Reconstruct the protected key
            let protected_key = EncryptedSymmetricKey::import(protected_key_string)
                .map_err(|_| anyhow::anyhow!("crypt error!"))?;
            // Decrypt the SymmetricKey using the PrivateKey
            let symmetric_key = protected_key
                .decrypt_with(private_key)
                .await
                .map_err(|_| anyhow::anyhow!("crypt error!"))?;
            // Create TemporalKey from SymmetrciKey
            let temporal_key = TemporalKey(AesKey::new(symmetric_key.as_ref().try_into()?));
            // Return
            Ok(temporal_key)
        } else {
            Err(KeyError::Missing.into())
        }
    }
}

impl Mapper {
    pub(crate) fn to_ipld(&self) -> Ipld {
        // New Map
        let mut map = BTreeMap::<String, Ipld>::new();
        // For each key value pair in the struct
        for (fingerprint, (public_key, encrypted_key)) in self.0.clone() {
            let mut sub_map = BTreeMap::<String, Ipld>::new();
            // Overwrite with fake data if there is no encrypted key
            let encrypted_key = if encrypted_key.len() == 242 {
                encrypted_key
            } else {
                String::from_utf8(vec![b' '; 242]).unwrap()
            };
            println!("encrypted key: {}", encrypted_key.len());
            // Insert the fingerprint
            sub_map.insert("public_key".to_string(), Ipld::Bytes(public_key));
            // Insert the encrypted key
            sub_map.insert("encrypted_key".to_string(), Ipld::String(encrypted_key));
            // Insert the sub_map
            map.insert(fingerprint, Ipld::Map(sub_map));
        }
        // Return
        Ipld::Map(map)
    }

    pub(crate) fn from_ipld(ipld: Ipld) -> Result<Self> {
        // New Mapper
        let mut mapper = Mapper::default();

        // If we can get the Map
        if let Ipld::Map(map) = ipld {
            // For each key value pair in the IPLD
            for (fingerprint, ipld) in map {
                if let Ipld::Map(sub_map) = ipld &&
                    let Some(Ipld::Bytes(public_key)) = sub_map.get("public_key") &&
                    let Some(Ipld::String(encrypted_key)) = sub_map.get("encrypted_key") {
                    // Use empty array if it is actually blank
                    let encrypted_key = if encrypted_key.as_bytes() == [b' '; 242] { String::new() } else { encrypted_key.to_string() };
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

impl std::fmt::Debug for Mapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Mapper").field(&self.0.keys()).finish()
    }
}

#[cfg(test)]
mod test {
    use super::Mapper;
    use anyhow::Result;
    use tomb_crypt::prelude::{EcEncryptionKey, WrappingPrivateKey};
    use wnfs::{
        common::dagcbor,
        private::{AesKey, TemporalKey},
    };

    #[tokio::test]
    async fn to_from_ipld() -> Result<()> {
        // Create new mapper
        let mut mapper1 = Mapper::default();
        // Create a new EC encryption key intended to be used to encrypt/decrypt temporal keys
        let wrapping_key = EcEncryptionKey::generate().await?;
        // Public Key
        let public_key = wrapping_key.public_key()?;
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
        // Create a new EC encryption key intended to be used to encrypt/decrypt temporal keys
        let wrapping_key = EcEncryptionKey::generate().await?;
        // Public Key
        let public_key = wrapping_key.public_key()?;
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
