use anyhow::Result;
use std::{
    fs::{File, OpenOptions},
    path::Path,
    io::{BufReader, Read, Write},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Wrapper for compression information
struct CompressionScheme {
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

/// Compresses bytes
pub fn compress_bytes<R, W>(reader: R, writer: W) -> Result<()>
where
    R: Read,
    W: Write,
{
    Ok(CompressionScheme::new_zstd().encode(reader, writer)?)
}

/// Grab a read-only reference to a file
pub fn get_read(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new().read(true).open(path)
}

/// Grab a write-only reference to a file
pub fn get_write(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new().append(false).write(true).open(path)
}

/// Get a read-write reference to a File on disk
pub fn get_read_write(path: &Path) -> Result<File, std::io::Error> {
    OpenOptions::new()
        .append(false)
        .read(true)
        .write(true)
        .open(path)
}

/// Compress the contents of a vector of bytes
pub fn compress_vec(buf: &[u8]) -> Result<Vec<u8>> {
    // Create a reader for the original file
    let reader = BufReader::new(buf);
    // Create a buffer to hold the compressed bytes
    let mut compressed: Vec<u8> = vec![];
    // Compress the chunk before feeding it to WNFS
    compress_bytes(reader, &mut compressed)?;
    // Return compressed bytes
    Ok(compressed)
}
