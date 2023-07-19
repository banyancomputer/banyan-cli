#[allow(unused_variables)]
mod common;

#[cfg(all(not(target_arch = "wasm32"), feature = "native"))]
mod native;

#[cfg(all(not(target_arch = "wasm32"), feature = "native"))]
pub use native::*;

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
mod wasm;

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub use wasm::*;

pub fn generate_info(encrypt_fingerprint_bytes: &[u8], decrypt_fingerprint_bytes: &[u8]) -> String {
    format!(
        "use=key_seal,encryptor={},decryptor={}",
        pretty_fingerprint(encrypt_fingerprint_bytes),
        pretty_fingerprint(decrypt_fingerprint_bytes),
    )
}

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

    const PLAINTEXT_SYMMETRIC_KEY: &[u8; 32] = b"demo-key-do-not-reuse-sample-key";

    const TEST_PEM_KEY: &[u8] = &[
        45, 45, 45, 45, 45, 66, 69, 71, 73, 78, 32, 80, 82, 73, 86, 65, 84, 69, 32, 75, 69, 89, 45,
        45, 45, 45, 45, 10, 77, 73, 71, 50, 65, 103, 69, 65, 77, 66, 65, 71, 66, 121, 113, 71, 83,
        77, 52, 57, 65, 103, 69, 71, 66, 83, 117, 66, 66, 65, 65, 105, 66, 73, 71, 101, 77, 73, 71,
        98, 65, 103, 69, 66, 66, 68, 67, 114, 72, 75, 99, 85, 106, 122, 102, 84, 102, 49, 105, 48,
        47, 80, 79, 53, 10, 49, 121, 111, 56, 102, 97, 97, 89, 112, 89, 100, 84, 105, 70, 81, 81,
        65, 111, 100, 85, 75, 113, 103, 86, 106, 70, 107, 83, 43, 98, 75, 66, 50, 112, 122, 86,
        118, 85, 122, 90, 68, 117, 120, 68, 84, 109, 54, 104, 90, 65, 78, 105, 65, 65, 82, 43, 120,
        74, 89, 50, 82, 82, 68, 88, 10, 88, 72, 55, 50, 112, 89, 74, 85, 54, 79, 81, 65, 80, 102,
        74, 110, 80, 86, 53, 97, 115, 121, 112, 87, 72, 100, 54, 98, 104, 56, 75, 50, 81, 77, 101,
        49, 117, 77, 73, 97, 105, 71, 70, 85, 117, 90, 84, 75, 71, 111, 121, 112, 48, 81, 72, 84,
        67, 117, 107, 111, 115, 68, 117, 69, 10, 76, 52, 101, 48, 52, 51, 74, 50, 103, 115, 101,
        119, 65, 85, 82, 80, 49, 115, 111, 69, 101, 105, 104, 103, 109, 97, 81, 112, 55, 73, 68,
        72, 115, 67, 56, 70, 107, 108, 66, 103, 71, 74, 90, 68, 112, 56, 100, 108, 116, 99, 117,
        89, 84, 110, 77, 61, 10, 45, 45, 45, 45, 45, 69, 78, 68, 32, 80, 82, 73, 86, 65, 84, 69,
        32, 75, 69, 89, 45, 45, 45, 45, 45, 10,
    ];

    const TEST_DER_KEY: &[u8] = &[
        48, 129, 164, 2, 1, 1, 4, 48, 171, 28, 167, 20, 143, 55, 211, 127, 88, 180, 252, 243, 185,
        215, 42, 60, 125, 166, 152, 165, 135, 83, 136, 84, 16, 2, 135, 84, 42, 168, 21, 140, 89,
        18, 249, 178, 129, 218, 156, 213, 189, 76, 217, 14, 236, 67, 78, 110, 160, 7, 6, 5, 43,
        129, 4, 0, 34, 161, 100, 3, 98, 0, 4, 126, 196, 150, 54, 69, 16, 215, 92, 126, 246, 165,
        130, 84, 232, 228, 0, 61, 242, 103, 61, 94, 90, 179, 42, 86, 29, 222, 155, 135, 194, 182,
        64, 199, 181, 184, 194, 26, 136, 97, 84, 185, 148, 202, 26, 140, 169, 209, 1, 211, 10, 233,
        40, 176, 59, 132, 47, 135, 180, 227, 114, 118, 130, 199, 176, 1, 68, 79, 214, 202, 4, 122,
        40, 96, 153, 164, 41, 236, 128, 199, 176, 47, 5, 146, 80, 96, 24, 150, 67, 167, 199, 101,
        181, 203, 152, 78, 115,
    ];

    const SEALED_KEY: &str = "GvQwdfPJ97rUTOl/UUHWjw==.knWedkNCmB11L2uRjpj6tU60mQs25kVSvCYMxDWiR9HKPgeR2sgISw==.MHYwEAYHKoZIzj0CAQYFK4EEACIDYgAECa67CCuaPgE+CuGb7acOFKdnzYy9I5hbU3AOQmi4clGAcmd9VAm+JeQqbz8mB1wwJQm1jhpYgcAjwC+kEPL9W2pneRNWwSm0lv15h2G0Jo8mA1NJUu7MDTFRNZQlGJf0";

    async fn test_pem_key_parse_and_use() -> Result<(), KeySealError> {
        use crate::key_seal::common::{ProtectedKey, WrappingPrivateKey};

        let private_key = EcEncryptionKey::import(TEST_PEM_KEY).await?;
        let protected_key = EncryptedSymmetricKey::import(SEALED_KEY)?;
        let plain_key = protected_key.decrypt_with(&private_key).await?;
        assert_eq!(plain_key.as_ref(), PLAINTEXT_SYMMETRIC_KEY);

        Ok(())
    }

    async fn test_der_key_parse_and_use() -> Result<(), KeySealError> {
        use crate::key_seal::common::{ProtectedKey, WrappingPrivateKey};

        let private_key = EcEncryptionKey::import_bytes(TEST_DER_KEY).await?;
        let protected_key = EncryptedSymmetricKey::import(SEALED_KEY)?;
        let plain_key = protected_key.decrypt_with(&private_key).await?;
        assert_eq!(plain_key.as_ref(), PLAINTEXT_SYMMETRIC_KEY);
        Ok(())
    }

    // this is a temporary test to ensure the end to end bits are working as expected while proper
    // tests are built
    async fn test_end_to_end() -> Result<(), KeySealError> {
        use crate::key_seal::common::{PlainKey, ProtectedKey, WrappingPrivateKey};

        let temporal_key = SymmetricKey::from(*PLAINTEXT_SYMMETRIC_KEY);

        let device_key = EcEncryptionKey::generate().await?;
        let encrypted_temporal_key = temporal_key.encrypt_for(&device_key.public_key()?).await?;
        let kex_blob = encrypted_temporal_key.export();

        let loaded_temporal_key = EncryptedSymmetricKey::import(&kex_blob)?;
        let decrypted_temporal_key = loaded_temporal_key.decrypt_with(&device_key).await?;

        let mut raw_temporal_key = [0u8; 32];
        raw_temporal_key.copy_from_slice(decrypted_temporal_key.as_ref());

        assert_eq!(PLAINTEXT_SYMMETRIC_KEY, &raw_temporal_key);

        Ok(())
    }

    // this is a temporary test to ensure the end to end bits are working as expected while proper
    // tests are built
    async fn test_key_roundtripping() -> Result<(), KeySealError> {
        use crate::key_seal::common::{WrappingPrivateKey, WrappingPublicKey};

        let key = EcEncryptionKey::generate().await?;
        let public_key = key.public_key()?;

        // dirty comparisons but works for now
        let raw_key_bytes = key.export_bytes().await?;
        let imported_key = EcEncryptionKey::import_bytes(&raw_key_bytes).await?;
        let reexported_key_bytes = imported_key.export_bytes().await?;
        assert_eq!(raw_key_bytes, reexported_key_bytes);

        let raw_public_key_bytes = public_key.export_bytes().await?;
        let imported_public_key =
            EcPublicEncryptionKey::import_bytes(&raw_public_key_bytes).await?;
        let reexported_public_key_bytes = imported_public_key.export_bytes().await?;
        assert_eq!(raw_public_key_bytes, reexported_public_key_bytes);

        let raw_key_pem = key.export().await?;
        let imported_key = EcEncryptionKey::import(&raw_key_pem).await?;
        let reexported_key_pem = imported_key.export().await?;
        assert_eq!(raw_key_pem, reexported_key_pem);

        let raw_public_key_pem = public_key.export().await?;
        let imported_public_key = EcPublicEncryptionKey::import(&raw_public_key_pem).await?;
        let reexported_public_key_pem = imported_public_key.export().await?;
        assert_eq!(raw_public_key_pem, reexported_public_key_pem);

        Ok(())
    }

    #[cfg(all(not(target_arch = "wasm32"), feature = "native"))]
    mod native_tests {
        use super::*;

        #[tokio::test]
        async fn pem_key_parse_and_use() -> Result<(), KeySealError> {
            test_pem_key_parse_and_use().await
        }

        #[tokio::test]
        async fn der_key_parse_and_use() -> Result<(), KeySealError> {
            test_der_key_parse_and_use().await
        }

        #[tokio::test]
        async fn end_to_end() -> Result<(), KeySealError> {
            test_end_to_end().await
        }

        #[tokio::test]
        async fn key_roundtripping() -> Result<(), KeySealError> {
            test_key_roundtripping().await
        }
    }

    #[cfg(all(target_arch = "wasm32", feature = "native"))]
    mod wasm_tests {
        use super::*;
        use wasm_bindgen_test::*;

        wasm_bindgen_test_configure!(run_in_browser);

        #[wasm_bindgen_test]
        async fn end_to_end() -> Result<(), KeySealError> {
            test_end_to_end().await
        }

        #[wasm_bindgen_test]
        async fn key_roundtripping() -> Result<(), KeySealError> {
            test_key_roundtripping().await
        }
    }
}
