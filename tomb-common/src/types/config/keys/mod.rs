pub mod manager;
pub mod mapper;
pub mod error;

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use rand::Rng;
    use serial_test::serial;
    use wnfs::private::{TemporalKey, AesKey, RsaPrivateKey};
    use crate::types::config::{globalconfig::GlobalConfig, keys::manager::Manager};

    fn random_temporal_key() -> TemporalKey {
        let random_bytes = rand::thread_rng().gen::<[u8; 32]>();
        TemporalKey(AesKey::new(random_bytes))
    }

    fn grab_rsa_private_key() -> Result<RsaPrivateKey> {
        let global = GlobalConfig::from_disk()?;
        global.wrapping_key_from_disk()
    }

    #[tokio::test]
    #[serial]
    async fn put_get_original() -> Result<()> {
        // Key manager
        let key_manager = Manager::default();

        // Grab rsa private
        let wrapping_key = grab_rsa_private_key()?;
        // Insert public key
        key_manager.insert(&wrapping_key.get_public_key()).await?;

        // Original TemporalKey
        let original = random_temporal_key();

        // Set the original key
        key_manager.set_original_key(&original).await?;

        let reconstructed_original = key_manager.retrieve_original(&wrapping_key).await?;

        // Assert that the original and reconstructed are matching
        assert_eq!(original, reconstructed_original);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn put_get_current() -> Result<()> {
        // Key manager
        let key_manager = Manager::default();

        // Grab rsa private
        let wrapping_key = grab_rsa_private_key()?;
        // Insert public key
        key_manager.insert(&wrapping_key.get_public_key()).await?;

        // current TemporalKey
        let current = random_temporal_key();

        // Set the current key
        key_manager.update_temporal_key(&current).await?;

        let reconstructed_current = key_manager.retrieve_current(&wrapping_key).await?;

        // Assert that the current and reconstructed are matching
        assert_eq!(current, reconstructed_current);

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn post_update_insertion() -> Result<()> {
        // Key manager
        let key_manager = Manager::default();

        // Grab RsaPrivateKey
        let wrapping_key = grab_rsa_private_key()?;
        // Grab temporal keys
        let original = random_temporal_key();
        let current = random_temporal_key();
        // Set the both keys
        key_manager.set_original_key(&current).await?;
        key_manager.update_temporal_key(&current).await?;

        // Insert public key post-hoc
        key_manager.insert(&wrapping_key.get_public_key()).await?;

        // Reconstruct the keys
        let reconstructed_original = key_manager.retrieve_original(&wrapping_key).await?;
        let reconstructed_current = key_manager.retrieve_current(&wrapping_key).await?;

        // Assert that the current and reconstructed keys are matching
        assert_eq!(original, reconstructed_original);
        assert_eq!(current, reconstructed_current);

        Ok(())
    }

}