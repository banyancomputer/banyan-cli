use age::secrecy::ExposeSecret;
//use core::num::dec2flt::parse;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Wrapper for compression information
pub struct CompressionScheme {
    /// The type of compression that is being used
    pub compression_info: String,
}

impl CompressionScheme {
    /// Creates a new `CompressionScheme` struct, specifying the ZSTD compression algorithm
    pub fn new_zstd() -> Self {
        CompressionScheme {
            compression_info: String::from("ZSTD"),
        }
    }

    /// Encode a file using the compression algorithm specified in the `CompressionScheme` struct
    pub fn encode<R, W>(&self, source: R, destination: W) -> Result<(), std::io::Error>
    where
        R: std::io::Read,
        W: std::io::Write,
    {
        match self.compression_info.as_str() {
            "ZSTD" => zstd::stream::copy_encode(source, destination, 1),
            _ => panic!("unsupported compression algorithm!"),
        }
    }

    /// Decode a file using the compression algorithm specified in the `CompressionScheme` struct
    pub fn decode<R, W>(&self, source: R, destination: W) -> Result<(), std::io::Error>
    where
        R: std::io::Read,
        W: std::io::Write,
    {
        match self.compression_info.as_str() {
            "ZSTD" => zstd::stream::copy_decode(source, destination),
            _ => panic!("unsupported compression algorithm!"),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Wrapper for partitioning information
pub struct PartitionScheme {
    /// Maximum packing chunk size
    pub chunk_size: u64,
}

#[derive(Clone, Serialize, Deserialize)]
/// Wrapper for encryption key information
pub struct EncryptionScheme {
    #[serde(
        serialize_with = "serialize_age_identity",
        deserialize_with = "deserialize_age_identity"
    )]
    pub(crate) identity: age::x25519::Identity,
}

impl std::fmt::Debug for EncryptionScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "encryption plans are secret for now")
    }
}

impl EncryptionScheme {
    /// Generate a new, unique encryption scheme using the default age encryption algorithm
    pub fn new_age() -> Self {
        EncryptionScheme {
            identity: age::x25519::Identity::generate(),
        }
    }
}

// Represent an age encryption key as a serialized string
fn serialize_age_identity<S>(
    identity: &age::x25519::Identity,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // Expose the secret key as a string
    serializer.serialize_str(identity.to_string().expose_secret())
}

// Reconstruct an age encryption struct from the serialized string counterpart
fn deserialize_age_identity<'de, D>(deserializer: D) -> Result<age::x25519::Identity, D::Error>
where
    D: serde::Deserializer<'de>,
{
    // Deserialize the key
    let key = String::deserialize(deserializer).map_err(|_| {
        serde::de::Error::custom("failed to deserialize age encryption key as a string")
    })?;
    // Construct from the string
    age::x25519::Identity::from_str(&key).map_err(serde::de::Error::custom)
}
