/// Implementation of Encrypted Private Ref
pub mod enc_ref;
/// Errors that can be encountered in these utils
pub mod error;
/// Manages original and current TemporalKeys
pub mod manager;
/// Maps key fingerprints to RsaPublicKeys and encrypted TemporalKeys
pub mod mapper;

#[cfg(test)]
mod test {
    use crate::share::manager::ShareManager;
    use anyhow::Result;
    use rand::Rng;
    use serial_test::serial;
    use tomb_crypt::prelude::*;
    use wnfs::private::{AesKey, TemporalKey, PrivateRef};

    fn random_private_ref() -> PrivateRef {
        let random_bytes = rand::thread_rng().gen::<[u8; 32]>();
        PrivateRef { 
            saturated_name_hash: random_bytes,
            temporal_key: TemporalKey::from(AesKey::new(random_bytes)),
            content_cid: Default::default(),
        }
    }

    #[tokio::test]
    #[serial]
    async fn put_get_original() -> Result<()> {
        // Key manager
        let mut key_manager = ShareManager::default();
        // Create a new EC encryption key intended to be used to encrypt/decrypt temporal keys
        let wrapping_key = EcEncryptionKey::generate().await?;
        // Public Key
        let public_key = wrapping_key.public_key()?;

        // Insert public key
        key_manager.share_with(&public_key).await?;

        // Original TemporalKey
        let original = random_private_ref();

        // Set the original key
        key_manager.set_original_ref(&original).await?;

        let reconstructed_original = key_manager.original_ref.ok_or(anyhow::anyhow!("No original ref"))?;

        // Assert that the original and reconstructed are matching
        assert_eq!(original, reconstructed_original);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn put_get_current() -> Result<()> {
        // Key manager
        let mut key_manager = ShareManager::default();
        // Create a new EC encryption key intended to be used to encrypt/decrypt temporal keys
        let wrapping_key = EcEncryptionKey::generate().await?;
        // Public Key
        let public_key = wrapping_key.public_key()?;
        // Insert public key
        key_manager.share_with(&public_key).await?;

        // current TemporalKey
        let current = random_private_ref();

        // Set the current key
        key_manager.set_current_ref(&current).await?;

        let reconstructed_current = key_manager.current_ref.ok_or(anyhow::anyhow!("No current ref"))?;

        // Assert that the current and reconstructed are matching
        assert_eq!(current, reconstructed_current);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn share_with_get_current() -> Result<()> {
        // Key manager
        let mut key_manager = ShareManager::default();
        // Create a new EC encryption key intended to be used to encrypt/decrypt temporal keys
        let wrapping_key = EcEncryptionKey::generate().await?;
        // Public Key
        let public_key = wrapping_key.public_key()?;
        // Grab temporal keys
        let current = random_private_ref();
        // Set the current key
        key_manager.set_current_ref(&current).await?;
        // Insert public key post-hoc
        key_manager.share_with(&public_key).await?;
        // Reconstruct the key
        let reconstructed_current = key_manager.current_ref.ok_or(anyhow::anyhow!("No current ref"))?;
        // Assert that the current and reconstructed keys are matching
        assert_eq!(current, reconstructed_current);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn share_with_get_original() -> Result<()> {
        // Key manager
        let mut key_manager = ShareManager::default();
        // Create a new EC encryption key intended to be used to encrypt/decrypt temporal keys
        let wrapping_key = EcEncryptionKey::generate().await?;
        // Public Key
        let public_key = wrapping_key.public_key()?;
        // Grab temporal keys
        let original = random_private_ref();
        // Set the current key
        key_manager.set_original_ref(&original).await?;
        // Insert public key post-hoc
        key_manager.share_with(&public_key).await?;
        // Reconstruct the key
        let reconstructed_original = key_manager.original_ref.ok_or(anyhow::anyhow!("No original ref"))?;
        // Assert that the current and reconstructed keys are matching
        assert_eq!(original, reconstructed_original);
        Ok(())
    }

    // #[tokio::test]
    // #[serial]
    // async fn share_with_get_both() -> Result<()> {
    //     // Key manager
    //     let mut key_manager = ShareManager::default();

    //     // Create a new EC encryption key intended to be used to encrypt/decrypt temporal keys
    //     let wrapping_key = EcEncryptionKey::generate().await?;
    //     // Public Key
    //     let public_key = wrapping_key.public_key()?;
    //     // Grab temporal keys
    //     let original = random_private_ref();
    //     let current = random_private_ref();

    //     // Set the both keys
    //     key_manager.set_original_key(&original).await?;
    //     key_manager.update_current_key(&current).await?;

    //     // Insert public key post-hoc
    //     key_manager.insert(&public_key).await?;

    //     // Reconstruct the keys
    //     let reconstructed_original = key_manager.retrieve_original(&wrapping_key).await?;
    //     let reconstructed_current = key_manager.retrieve_current(&wrapping_key).await?;

    //     // Assert that the current and reconstructed keys are matching
    //     assert_eq!(original, reconstructed_original);
    //     assert_eq!(current, reconstructed_current);

    //     Ok(())
    // }
}
