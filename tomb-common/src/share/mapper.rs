use super::error::KeyError;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tomb_crypt::prelude::*;
use tomb_crypt::pretty_fingerprint;
use wnfs::private::AccessKey;

use libipld::Ipld;
use std::collections::{BTreeMap, HashMap};

use crate::share::enc_key::EncryptedAccessKey;

const PUBLIC_KEY_LABEL: &str = "PUBLIC_KEY";
const ENCRYPTED_PRIVATE_REF_LABEL: &str = "ENCRYPTED_PRIVATE_REF";

#[derive(Default, PartialEq, Debug, Clone)]
/// Map of:
/// ECDH fingerprint -> (ECDH_PUBLIC_KEY, ENC_PRIVATE_REF)
/// where
///  - ECDH_PUBLIC_KEY is the DER encoded public key bytes of an ECDH keypair
///  - ENC_PRIVATE_REF is an EncryptedPrivateRef shared with that public key
pub struct EncryptedKeyMapper(pub(crate) HashMap<String, (Vec<u8>, String)>);

impl EncryptedKeyMapper {
    /// Encrypt a private referece for all reciepients in the Mapper
    pub async fn update_ref(&mut self, access_key: &AccessKey) -> Result<()> {
        // For each Public Key present in the map
        for (fingerprint, (der, _)) in self.0.clone() {
            // Get the public key from the DER
            let public_key = EcPublicEncryptionKey::import_bytes(&der)
                .await
                .map_err(|_| anyhow::anyhow!("could not import recipient key"))?;
            // Encrypt the private ref for the public key
            let encrypted_private_ref = EncryptedAccessKey::encrypt_for(access_key, &public_key)
                .await
                .map_err(|_| anyhow::anyhow!("could not encrypt private ref for recipient"))?;
            // Insert the encrypted private ref into the map
            let ref_string = serde_json::to_string(&encrypted_private_ref)
                .map_err(|_| anyhow::anyhow!("could not export encrypted private ref to string"))?;
            // Insert the reencrypted version of the TemporalKey
            self.0.insert(fingerprint, (der, ref_string));
        }
        Ok(())
    }

    /// Add a new recipient to the mapper
    /// Optionally share a private ref with the new recipient key
    pub async fn add_recipient(
        &mut self,
        access_key: &Option<AccessKey>,
        recipient: &EcPublicEncryptionKey,
    ) -> Result<()> {
        // Grab the public key's fingerprint
        let fingerprint = pretty_fingerprint(
            &recipient
                .fingerprint()
                .await
                .map_err(|_| anyhow::anyhow!("could not fingerprint recipient"))?,
        );
        // Get the DER encoded public key bytes
        let der = recipient
            .export_bytes()
            .await
            .map_err(|_| anyhow::anyhow!("could not export recipient public key to DER"))?;

        // If there is a valid temporal key
        let ref_string = match access_key {
            Some(access_key) => {
                // Encrypt the access key for the recipient
                let encrypted_access_key = EncryptedAccessKey::encrypt_for(access_key, recipient)
                    .await
                    .map_err(|_| anyhow::anyhow!("could not encrypt private ref for recipient"))?;
                // Export the encrypted private ref to a string
                // Insert the encrypted private ref into the map
                serde_json::to_string(&encrypted_access_key).map_err(|_| {
                    anyhow::anyhow!("could not export encrypted private ref to string")
                })?
            }
            None => String::new(),
        };
        // Insert into the hashmap, using fingerprint as key
        self.0.insert(fingerprint, (der, ref_string));
        // Ok
        Ok(())
    }

    /// Decrypt the TemporalKey using a recipient's PrivateKey
    pub async fn recover_ref(&self, recipient: &EcEncryptionKey) -> Result<AccessKey> {
        // Grab the fingerprint from the
        let fingerprint = pretty_fingerprint(
            &recipient
                .fingerprint()
                .await
                .map_err(|_| anyhow::anyhow!("could not fingerprint recipient"))?,
        );
        // Grab the encrypted key associated with the fingerprint
        let (_, enc_key_string) = match self.0.get(&fingerprint) {
            Some(entry) => entry,
            None => return Err(KeyError::Missing.into()),
        };
        let enc_key = serde_json::from_str::<EncryptedAccessKey>(enc_key_string)
            .map_err(|_| anyhow::anyhow!("could not deserialize encrypted private ref"))?;
        let private_ref = enc_key
            .decrypt_with(recipient)
            .await
            .map_err(|_| anyhow::anyhow!("could not decrypt private ref"))?;
        Ok(private_ref)
    }
}

impl EncryptedKeyMapper {
    pub(crate) fn to_ipld(&self) -> Ipld {
        // New Map
        let mut map = BTreeMap::<String, Ipld>::new();
        // For each key value pair in the struct
        for (fingerprint, (public_key, encrypted_private_ref)) in self.0.clone() {
            let mut sub_map = BTreeMap::<String, Ipld>::new();
            // Insert the fingerprint
            sub_map.insert(PUBLIC_KEY_LABEL.to_string(), Ipld::Bytes(public_key));
            // Insert the encrypted key
            sub_map.insert(
                ENCRYPTED_PRIVATE_REF_LABEL.to_string(),
                Ipld::String(encrypted_private_ref),
            );
            // Insert the sub_map
            map.insert(fingerprint, Ipld::Map(sub_map));
        }
        // Return
        Ipld::Map(map)
    }

    pub(crate) fn from_ipld(ipld: Ipld) -> Result<Self> {
        // New EncryptedKeyMapper
        let mut mapper = EncryptedKeyMapper::default();
        // If we can get the Map
        if let Ipld::Map(map) = ipld {
            // For each key value pair in the IPLD
            for (fingerprint, ipld) in map {
                if let Ipld::Map(sub_map) = ipld &&
                    let Some(Ipld::Bytes(public_key)) = sub_map.get(PUBLIC_KEY_LABEL) &&
                    let Some(Ipld::String(encrypted_private_ref)) = sub_map.get(ENCRYPTED_PRIVATE_REF_LABEL) {
                    // Insert the new value into the mapper
                    mapper.0.insert(fingerprint, (public_key.to_vec(), encrypted_private_ref.to_string()));
                }
                else {
                    return Err(KeyError::Missing.into());
                }
            }
        }
        Ok(mapper)
    }
}

impl Serialize for EncryptedKeyMapper {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_ipld().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EncryptedKeyMapper {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ipld = Ipld::deserialize(deserializer)?;
        Ok(Self::from_ipld(ipld).expect("failed to convert IPLD to EncryptedKeyMapper"))
    }
}

/*
#[cfg(test)]
mod test {
    use super::*;
    use anyhow::Result;
    use wnfs::private::{TemporalKey, TemporalAccessKey};
    use libipld::Cid;

    #[tokio::test]
    async fn to_from_ipld() -> Result<()> {
        // Create new mapper
        let mut mapper1 = EncryptedKeyMapper::default();
        // Create a new EC encryption key intended to be used to encrypt/decrypt temporal keys
        let wrapping_key = EcEncryptionKey::generate().await?;
        // Public Key
        let public_key = wrapping_key.public_key()?;
        // Insert a public key
        mapper1.add_recipient(&None, &public_key).await?;

        let mapper1_ipld = mapper1.to_ipld();
        let mut mapper2 = EncryptedKeyMapper::from_ipld(mapper1_ipld)?;
        // Assert reconstruction
        assert_eq!(mapper1, mapper2);
        let temporal_key = TemporalKey([7u8; 32]);
        let private_ref = AccessKey::Temporal(TemporalAccessKey { label: [0u8; 32], content_cid: Cid::default(), temporal_key });

        // Update temporal key
        mapper2.update_ref(&private_ref).await?;

        let mapper2_ipld = mapper2.to_ipld();
        let mapper3 = EncryptedKeyMapper::from_ipld(mapper2_ipld)?;
        // Assert reconstruction
        assert_eq!(mapper2, mapper3);

        // Assert decryption
        let new_private_ref = mapper3.recover_ref(&wrapping_key).await?;
        assert_eq!(private_ref, new_private_ref);

        Ok(())
    }

    #[tokio::test]
    async fn serial_size() -> Result<()> {
        // Create new mapper
        let mut mapper1 = EncryptedKeyMapper::default();
        // Create a new EC encryption key intended to be used to encrypt/decrypt temporal keys
        let wrapping_key = EcEncryptionKey::generate().await?;
        // Public Key
        let public_key = wrapping_key.public_key()?;
        // Insert a public key
        mapper1.add_recipient(&None, &public_key).await?;

        // Serialize
        let mapper1_bytes = serde_json::to_vec(&mapper1)?;
        let mut mapper2: EncryptedKeyMapper = serde_json::from_slice(&mapper1_bytes.as_slice())?;
        // Assert reconstruction
        assert_eq!(mapper1, mapper2);

        let temporal_key = TemporalKey(AesKey::new([7u8; 32]));
        let private_ref = PrivateRef {
            temporal_key: temporal_key.clone(),
            saturated_name_hash: [0u8; 32],
            content_cid: Cid::default(),
        };

        // Update temporal key
        mapper2.update_ref(&private_ref).await?;

        let mapper2_bytes = serde_json::to_vec(&mapper2)?;
        let mapper3: EncryptedKeyMapper = serde_json::from_slice(mapper2_bytes.as_slice())?;
        // Assert reconstruction
        assert_eq!(mapper2, mapper3);

        // Assert decryption
        let new_private_ref = mapper3.recover_ref(&wrapping_key).await?;
        assert_eq!(private_ref, new_private_ref);

        Ok(())
    }
}
 */
