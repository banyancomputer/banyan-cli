use crate::key_seal::KeySealError;

/// Number of bytes used for our AES keys (256-bit)
pub const AES_KEY_SIZE: usize = 32;

/// Length of a negotiated key exchange using our select EC curve (P384). It is assumed other
/// algorithms with different key lengths aren't going to be used.
pub const ECDH_SECRET_BYTE_SIZE: usize = 48;

/// Number of bytes present in an unformatted fingerprint.
pub const FINGERPRINT_SIZE: usize = 20;

/// Number of bytes used for our salts and IVs
pub const SALT_SIZE: usize = 16;

pub trait WrappingPrivateKey: Sized {
    type PublicKey: WrappingPublicKey;

    /// Converts the private key representation into a PEM wrapped PKCS8 private key. The returned
    /// bytes should all be printable UTF8 characters which can be turned into a string on demand.
    ///
    /// This format should be preferred if the data is going to be visible to people or platforms
    /// as it is immediately recognizable.
    fn export(&self) -> Result<Vec<u8>, KeySealError>;

    /// Export the internal private key into a DER encoded set of bytes.
    fn export_bytes(&self) -> Result<Vec<u8>, KeySealError>;

    fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError> {
        self.public_key()?.fingerprint()
    }

    fn generate() -> Result<Self, KeySealError>;
    fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError>;
    fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError>;
    fn public_key(&self) -> Result<Self::PublicKey, KeySealError>;
}

pub trait WrappingPublicKey: Sized {
    fn export(&self) -> Result<Vec<u8>, KeySealError>;
    fn export_bytes(&self) -> Result<Vec<u8>, KeySealError>;

    fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], KeySealError>;

    fn import(pem_bytes: &[u8]) -> Result<Self, KeySealError>;
    fn import_bytes(der_bytes: &[u8]) -> Result<Self, KeySealError>;
}

pub trait PlainKey: AsRef<[u8]> + From<[u8; AES_KEY_SIZE]> {
    type ProtectedKey: ProtectedKey;
    type WrappingPublicKey: WrappingPublicKey;

    fn encrypt_for(
        &self,
        recipient_key: &Self::WrappingPublicKey,
    ) -> Result<Self::ProtectedKey, KeySealError>;
}

pub trait ProtectedKey: Sized {
    type PlainKey: PlainKey;
    type WrappingPrivateKey: WrappingPrivateKey;

    fn decrypt_with(&self, recipient_key: &Self::WrappingPrivateKey) -> Result<Self::PlainKey, KeySealError>;
    fn export(&self) -> String;
    fn import(serialized: &str) -> Result<Self, KeySealError>;
}
