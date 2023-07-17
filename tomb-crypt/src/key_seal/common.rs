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

/// A WrappingPrivateKey is an opinionated cryptographic type designed for encrypting and
/// decrypting (wrapping) a symmetric AES key using an EC group key.
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

    /// Create a standards compliant SHA1 fingerprint of the associated public key encoded as a
    /// fixed length bytes string. This is usually presented to users by running it through the
    /// prettifier [`crate::key_seal::pretty_fingerprint()`].
    fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], Self::Error> {
        self.public_key()?.fingerprint()
    }

    /// Creates a secure new private key matching the security and use requirements for use as a EC
    /// wrapping key.
    fn generate() -> Result<Self, Self::Error>;

    /// Parses a PEM encoded EC private key into the internal type appropriate for being used as a
    /// wrapping key.
    fn import(pem_bytes: &[u8]) -> Result<Self, Self::Error>;

    /// Parses a DER encoded EC private key into the internal type appropriate for being used as a
    /// wrapping key.
    fn import_bytes(der_bytes: &[u8]) -> Result<Self, Self::Error>;

    /// Generates the public portion of this private key.
    fn public_key(&self) -> Result<Self::PublicKey, Self::Error>;
}

/// The public portion of a [`WrappingPrivateKey`]. The public portion is important for tracking
/// the identity of the keys and can be used to encrypt any plain key in a way the holder the
/// private key can get access to.
pub trait WrappingPublicKey: Sized {
    type Error: Error;

    /// Converts the public portion of the wrapping key into a PEM/SPKI formatted version that is
    /// easy to exchange in a visibly identifiable way and works over ASCII only channels.
    fn export(&self) -> Result<Vec<u8>, Self::Error>;

    /// Exports the public portion of a private key as a DER formatted byte string. Preferred when
    /// exchanging and embedding in formats that will already be encoded using other means.
    fn export_bytes(&self) -> Result<Vec<u8>, Self::Error>;

    /// Generates a SHA1 over the standardized compressed form representation of an EC key. This is
    /// usually presented to users by running it through the prettifier
    /// [`crate::key_seal::pretty_fingerprint()`].
    fn fingerprint(&self) -> Result<[u8; FINGERPRINT_SIZE], Self::Error>;

    /// IMPORT A STANDARD PEM FORMATTED VERSION OF AN EC KEY.
    fn import(pem_bytes: &[u8]) -> Result<Self, Self::Error>;

    /// Import a standard DER formatted EC key byte string
    fn import_bytes(der_bytes: &[u8]) -> Result<Self, Self::Error>;
}

/// A wrapper around an unprotected 256-bit AES key. The raw key can act as a raw byte string for
/// other implementation to use for encryption and decryption.
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

    fn decrypt_with(
        &self,
        recipient_key: &Self::WrappingPrivateKey,
    ) -> Result<Self::PlainKey, Self::Error>;
    fn export(&self) -> String;
    fn import(serialized: &str) -> Result<Self, Self::Error>;
}
