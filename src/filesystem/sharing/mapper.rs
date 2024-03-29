use super::SharingError;
use serde::{Deserialize, Serialize};
use tomb_crypt::{
    hex_fingerprint,
    prelude::{EcEncryptionKey, EcPublicEncryptionKey, PrivateKey, PublicKey},
};
use wnfs::private::PrivateRef;

use std::collections::{BTreeMap, HashMap};
use wnfs::libipld::Ipld;

use crate::{cast, filesystem::sharing::enc_ref::EncryptedPrivateRef};

const PUBLIC_KEY_LABEL: &str = "PUBLIC_KEY";
const ENCRYPTED_PRIVATE_REF_LABEL: &str = "ENCRYPTED_PRIVATE_REF";

#[derive(Default, PartialEq, Debug, Clone)]
/// Map of:
/// ECDH fingerprint -> (ECDH_PUBLIC_KEY, ENC_PRIVATE_REF)
/// where
///  - ECDH_PUBLIC_KEY is the DER encoded public key bytes of an ECDH keypair
///  - ENC_PRIVATE_REF is an EncryptedPrivateRef shared with that public key
pub struct EncRefMapper(pub(crate) HashMap<String, (Vec<u8>, String)>);

impl EncRefMapper {
    /// Encrypt a private referece for all reciepients in the Mapper
    pub async fn update_ref(&mut self, private_ref: &PrivateRef) -> Result<(), SharingError> {
        // For each Public Key present in the map
        for (fingerprint, (der, _)) in self.0.clone() {
            // Get the public key from the DER
            let public_key = EcPublicEncryptionKey::import_bytes(&der).await?;
            // Encrypt the private ref for the public key
            let encrypted_private_ref =
                EncryptedPrivateRef::encrypt_for(private_ref, &public_key).await?;
            // Insert the encrypted private ref into the map
            let ref_string = serde_json::to_string(&encrypted_private_ref)?;
            // Insert the reencrypted version of the TemporalKey
            self.0.insert(fingerprint, (der, ref_string));
        }
        Ok(())
    }

    /// Add a new recipient to the mapper
    /// Optionally share a private ref with the new recipient key
    pub async fn add_recipient(
        &mut self,
        private_ref: &Option<PrivateRef>,
        recipient: &EcPublicEncryptionKey,
    ) -> Result<(), SharingError> {
        // Grab the public key's fingerprint
        let fingerprint = hex_fingerprint(recipient.fingerprint().await?.as_slice());
        // Get the DER encoded public key bytes
        let der = recipient.export_bytes().await?;

        // If there is a valid temporal key
        let ref_string = match private_ref {
            Some(private_ref) => {
                // Encrypt the private ref for the recipient
                let encrypted_private_ref =
                    EncryptedPrivateRef::encrypt_for(private_ref, recipient).await?;
                // Export the encrypted private ref to a string
                serde_json::to_string(&encrypted_private_ref)?
            }
            None => String::new(),
        };
        // Insert into the hashmap, using fingerprint as key
        self.0.insert(fingerprint, (der, ref_string));
        // Ok
        Ok(())
    }

    /// Decrypt the TemporalKey using a recipient's PrivateKey
    pub async fn recover_ref(
        &self,
        recipient: &EcEncryptionKey,
    ) -> Result<PrivateRef, SharingError> {
        // Grab the fingerprint from the
        let fingerprint = hex_fingerprint(recipient.fingerprint().await?.as_slice());
        // Grab the encrypted key associated with the fingerprint
        let (_, enc_ref_string) = match self.0.get(&fingerprint) {
            Some(entry) => entry,
            None => return Err(SharingError::unauthorized()),
        };
        let enc_ref = serde_json::from_str::<EncryptedPrivateRef>(enc_ref_string)?;
        let private_ref = enc_ref.decrypt_with(recipient).await?;
        Ok(private_ref)
    }
}

impl EncRefMapper {
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

    pub(crate) fn from_ipld(ipld: Ipld) -> Result<Self, SharingError> {
        // New EncRefMapper
        let mut mapper = EncRefMapper::default();
        // If we can get the Map
        if let Ipld::Map(map) = ipld {
            // For each key value pair in the IPLD
            for (fingerprint, ipld) in map {
                // Get the expected variables, erroring if we fail
                let map = cast!(ipld, Ipld::Map).ok_or(SharingError::unauthorized())?;
                let bytes = map
                    .get(PUBLIC_KEY_LABEL)
                    .ok_or(SharingError::unauthorized())?;
                let string = map
                    .get(ENCRYPTED_PRIVATE_REF_LABEL)
                    .ok_or(SharingError::unauthorized())?;
                let public_key = cast!(bytes, Ipld::Bytes).ok_or(SharingError::unauthorized())?;
                let encrypted_private_ref =
                    cast!(string, Ipld::String).ok_or(SharingError::unauthorized())?;

                // Insert the new value into the mapper
                mapper.0.insert(
                    fingerprint,
                    (public_key.to_vec(), encrypted_private_ref.to_string()),
                );
            }
        }
        Ok(mapper)
    }
}

impl Serialize for EncRefMapper {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_ipld().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EncRefMapper {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ipld = Ipld::deserialize(deserializer)?;
        Ok(Self::from_ipld(ipld).expect("failed to convert IPLD to EncRefMapper"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod test {

    use tomb_crypt::prelude::{EcEncryptionKey, PrivateKey};
    use wnfs::{
        common::dagcbor,
        libipld::Cid,
        private::{AesKey, PrivateRef, TemporalKey},
    };

    use crate::{
        filesystem::sharing::mapper::EncRefMapper, prelude::filesystem::sharing::SharingError,
    };

    #[tokio::test]
    async fn to_from_ipld() -> Result<(), SharingError> {
        // Create new mapper
        let mut mapper1 = EncRefMapper::default();
        // Create a new EC encryption key intended to be used to encrypt/decrypt temporal keys
        let wrapping_key = EcEncryptionKey::generate().await?;
        // Public Key
        let public_key = wrapping_key.public_key()?;
        // Insert a public key
        mapper1.add_recipient(&None, &public_key).await?;

        let mapper1_ipld = mapper1.to_ipld();
        let mut mapper2 = EncRefMapper::from_ipld(mapper1_ipld)?;
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

        let mapper2_ipld = mapper2.to_ipld();
        let mapper3 = EncRefMapper::from_ipld(mapper2_ipld)?;
        // Assert reconstruction
        assert_eq!(mapper2, mapper3);

        // Assert decryption
        let new_private_ref = mapper3.recover_ref(&wrapping_key).await?;
        assert_eq!(private_ref, new_private_ref);

        Ok(())
    }

    #[tokio::test]
    async fn serial_size() -> Result<(), SharingError> {
        // Create new mapper
        let mut mapper1 = EncRefMapper::default();
        // Create a new EC encryption key intended to be used to encrypt/decrypt temporal keys
        let wrapping_key = EcEncryptionKey::generate().await?;
        // Public Key
        let public_key = wrapping_key.public_key()?;
        // Insert a public key
        mapper1.add_recipient(&None, &public_key).await?;

        // Serialize
        let mapper1_bytes = dagcbor::encode(&mapper1)
            .map_err(|err| SharingError::invalid_data(&err.to_string()))?;
        let mut mapper2: EncRefMapper = dagcbor::decode(mapper1_bytes.as_slice())
            .map_err(|err| SharingError::invalid_data(&err.to_string()))?;
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

        let mapper2_bytes = dagcbor::encode(&mapper2)
            .map_err(|err| SharingError::invalid_data(&err.to_string()))?;
        let mapper3: EncRefMapper = dagcbor::decode(mapper2_bytes.as_slice())
            .map_err(|err| SharingError::invalid_data(&err.to_string()))?;
        // Assert reconstruction
        assert_eq!(mapper2, mapper3);

        // Assert decryption
        let new_private_ref = mapper3.recover_ref(&wrapping_key).await?;
        assert_eq!(private_ref, new_private_ref);

        Ok(())
    }
}
