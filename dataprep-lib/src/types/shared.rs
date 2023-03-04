use age::secrecy::ExposeSecret;
//use core::num::dec2flt::parse;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionScheme {
    pub compression_info: &'static str,
}
impl CompressionScheme {
    pub fn new_zstd() -> Self {
        CompressionScheme {
            compression_info: "ZSTD",
        }
    }
    // pub fn get_encoder(&self, reader: impl Read) -> impl Read {
    //     match self.compression_info {
    //         "ZSTD" => zstd::Encoder::new(reader, 0).unwrap(),
    //         _ => panic!("unsupported compression!"),
    //     }
    // }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionScheme {
    /// The size of the chunks
    pub chunk_size: u64,
}

#[derive(Clone, Serialize, Deserialize)]
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
    pub fn new() -> Self {
        EncryptionScheme {
            identity: age::x25519::Identity::generate(),
        }
    }
}

impl Default for EncryptionScheme {
    fn default() -> Self {
        Self::new()
    }
}

fn serialize_age_identity<S>(
    identity: &age::x25519::Identity,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(identity.to_string().expose_secret())
}

fn deserialize_age_identity<'de, D>(deserializer: D) -> Result<age::x25519::Identity, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    age::x25519::Identity::from_str(&s).map_err(serde::de::Error::custom)
}
