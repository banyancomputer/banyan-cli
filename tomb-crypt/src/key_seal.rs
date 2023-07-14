mod common;

#[cfg(all(not(target_arch = "wasm"), feature = "standard"))]
mod standard;

#[cfg(all(not(target_arch = "wasm"), feature = "standard"))]
pub use standard::{EcEncryptionKey, EcPublicEncryptionKey, EncryptedSymmetricKey, KeySealError, SymmetricKey};

//#[cfg(target_arch = "wasm")]
//mod wasm;

//#[cfg(target_arch = "wasm")]
//{
//}

pub fn pretty_fingerprint(fingerprint_bytes: &[u8]) -> String {
    fingerprint_bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<String>>()
        .join(":")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_seal::common::*;

    // this is a temporary test to ensure the end to end bits are working as expected while proper
    // tests are built
    #[test]
    fn end_to_end_test() -> Result<(), KeySealError> {
        let plaintext_temporal_key: &[u8; 32] = b"demo-key-do-not-reuse-sample-key";
        let temporal_key = SymmetricKey::from(*plaintext_temporal_key);

        let our_device_key = EcEncryptionKey::generate()?;
        let our_encrypted_temporal_key = temporal_key.encrypt_for(&our_device_key.public_key()?)?;
        let our_kex_blob = our_encrypted_temporal_key.export();

        let loaded_temporal_key = EncryptedSymmetricKey::import(&our_kex_blob)?;
        let decrypted_temporal_key = loaded_temporal_key.decrypt_with(&our_device_key)?;

        let mut raw_temporal_key = [0u8; 32];
        raw_temporal_key.copy_from_slice(decrypted_temporal_key.as_ref());

        assert_eq!(plaintext_temporal_key, &raw_temporal_key);

        Ok(())
    }
}
