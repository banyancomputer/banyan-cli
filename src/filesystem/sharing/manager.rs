use super::{mapper::EncRefMapper, SharingError};
use serde::{Deserialize, Serialize};
use tomb_crypt::prelude::{EcEncryptionKey, EcPublicEncryptionKey};
use wnfs::private::PrivateRef;

/// Fs Share manager
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ShareManager {
    /// The unencrypted original PrivateRef
    pub original_ref: Option<PrivateRef>,
    /// The unencrypted current_ref PrivateRef
    pub current_ref: Option<PrivateRef>,
    /// EncRefMapper for original PrivateRef
    pub original_map: EncRefMapper,
    /// EncRefMapper for current_ref PrivateRef
    pub current_map: EncRefMapper,
}

impl ShareManager {
    /// Update the current_ref PrivateRef
    pub async fn set_current_ref(&mut self, new_ref: &PrivateRef) -> Result<(), SharingError> {
        // Update the PrivateRef
        self.current_ref = Some(new_ref.clone());
        self.current_map.update_ref(new_ref).await?;
        Ok(())
    }

    /// Update the original PrivateRef
    pub async fn set_original_ref(&mut self, new_ref: &PrivateRef) -> Result<(), SharingError> {
        // Update the PrivateRef
        self.original_ref = Some(new_ref.clone());
        self.original_map.update_ref(new_ref).await?;
        Ok(())
    }

    /// Share our references with a new recipient
    pub async fn share_with(
        &mut self,
        recipient: &EcPublicEncryptionKey,
    ) -> Result<(), SharingError> {
        // Insert into original
        self.original_map
            .add_recipient(&self.original_ref, recipient)
            .await?;

        // Insert into current
        self.current_map
            .add_recipient(&self.current_ref, recipient)
            .await?;

        Ok(())
    }

    /// Grab a list of the PEM strings for each Public Key recipient
    pub fn public_fingerprints(&self) -> Vec<String> {
        self.original_map.0.clone().into_keys().collect()
    }

    /// Retrieve the current_ref PrivateRef using a PrivateKey
    async fn current_ref(&self, recipient: &EcEncryptionKey) -> Result<PrivateRef, SharingError> {
        self.current_map.recover_ref(recipient).await
    }

    /// Retrieve the original PrivateRef using a PrivateKey
    async fn original_ref(&self, recipient: &EcEncryptionKey) -> Result<PrivateRef, SharingError> {
        self.original_map.recover_ref(recipient).await
    }

    /// Reload both refs into memory
    pub async fn load_refs(&mut self, recipient: &EcEncryptionKey) -> Result<(), SharingError> {
        self.current_ref = Some(self.current_ref(recipient).await?);
        self.original_ref = Some(self.original_ref(recipient).await?);
        Ok(())
    }
}

impl Serialize for ShareManager {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (&self.current_map, &self.original_map).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ShareManager {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (current_map, original_map) =
            <(EncRefMapper, EncRefMapper)>::deserialize(deserializer)?;
        Ok(Self {
            original_ref: None,
            current_ref: None,
            original_map,
            current_map,
        })
    }
}
