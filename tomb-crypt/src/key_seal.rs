mod common;

#[cfg(all(not(target_arch = "wasm"), feature = "native"))]
mod native;

#[cfg(all(not(target_arch = "wasm"), feature = "native"))]
pub use native::{
    EcEncryptionKey, EcPublicEncryptionKey, EncryptedSymmetricKey, KeySealError, SymmetricKey,
};

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

        let device_key = EcEncryptionKey::generate()?;
        let encrypted_temporal_key = temporal_key.encrypt_for(&device_key.public_key()?)?;
        let kex_blob = encrypted_temporal_key.export();

        let loaded_temporal_key = EncryptedSymmetricKey::import(&kex_blob)?;
        let decrypted_temporal_key = loaded_temporal_key.decrypt_with(&device_key)?;

        let mut raw_temporal_key = [0u8; 32];
        raw_temporal_key.copy_from_slice(decrypted_temporal_key.as_ref());

        assert_eq!(plaintext_temporal_key, &raw_temporal_key);

        Ok(())
    }

    // this is a temporary test to ensure the end to end bits are working as expected while proper
    // tests are built
    #[test]
    fn test_key_roundtripping() -> Result<(), KeySealError> {
        let key = EcEncryptionKey::generate()?;
        let public_key = key.public_key()?;

        // dirty comparisons but works for now
        let raw_key_bytes = key.export_bytes()?;
        let imported_key = EcEncryptionKey::import_bytes(&raw_key_bytes)?;
        let reexported_key_bytes = imported_key.export_bytes()?;
        assert_eq!(raw_key_bytes, reexported_key_bytes);

        let raw_public_key_bytes = public_key.export_bytes()?;
        let imported_public_key = EcPublicEncryptionKey::import_bytes(&raw_public_key_bytes)?;
        let reexported_public_key_bytes = imported_public_key.export_bytes()?;
        assert_eq!(raw_public_key_bytes, reexported_public_key_bytes);

        let raw_key_pem = key.export()?;
        let imported_key = EcEncryptionKey::import(&raw_key_pem)?;
        let reexported_key_pem = imported_key.export()?;
        assert_eq!(raw_key_pem, reexported_key_pem);

        let raw_public_key_pem = public_key.export()?;
        let imported_public_key = EcPublicEncryptionKey::import(&raw_public_key_pem)?;
        let reexported_public_key_pem = imported_public_key.export()?;
        assert_eq!(raw_public_key_pem, reexported_public_key_pem);

        Ok(())
    }
}
