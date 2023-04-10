use serde::{Deserialize, Serialize};

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
