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
