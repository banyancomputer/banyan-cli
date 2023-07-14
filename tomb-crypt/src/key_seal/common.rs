use std::error::Error;

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
    type Error: Error;
    type PublicKey: WrappingPublicKey<Error = Self::Error>;

    /// Converts the private key representation into a PEM wrapped PKCS8 private key. The returned
    /// bytes should all be printable UTF8 characters which can be turned into a string on demand.
    ///
    /// This format should be preferred if the data is going to be visible to people or platforms
    /// as it is immediately recognizable.
    fn export(&self) -> Result<Vec<u8>, Self::Error>;

    /// Export the internal private key into a DER encoded set of bytes.
    fn export_bytes(&self) -> Result<Vec<u8>, Self::Error>;

    fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], Self::Error> {
        self.public_key()?.fingerprint()
    }

    fn generate() -> Result<Self, Self::Error>;
    fn import(pem_bytes: &[u8]) -> Result<Self, Self::Error>;
    fn import_bytes(der_bytes: &[u8]) -> Result<Self, Self::Error>;
    fn public_key(&self) -> Result<Self::PublicKey, Self::Error>;
}

pub trait WrappingPublicKey: Sized {
    type Error: Error;

    fn export(&self) -> Result<Vec<u8>, Self::Error>;
    fn export_bytes(&self) -> Result<Vec<u8>, Self::Error>;

    fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], Self::Error>;

    fn import(pem_bytes: &[u8]) -> Result<Self, Self::Error>;
    fn import_bytes(der_bytes: &[u8]) -> Result<Self, Self::Error>;
}

pub trait PlainKey: AsRef<[u8]> + From<[u8; AES_KEY_SIZE]> {
    type Error: Error;
    type ProtectedKey: ProtectedKey;
    type WrappingPublicKey: WrappingPublicKey;

    fn encrypt_for(
        &self,
        recipient_key: &Self::WrappingPublicKey,
    ) -> Result<Self::ProtectedKey, Self::Error>;
}

pub trait ProtectedKey: Sized {
    type Error: Error;
    type PlainKey: PlainKey;
    type WrappingPrivateKey: WrappingPrivateKey;

    fn decrypt_with(&self, recipient_key: &Self::WrappingPrivateKey) -> Result<Self::PlainKey, Self::Error>;
    fn export(&self) -> String;
    fn import(serialized: &str) -> Result<Self, Self::Error>;
}
