use anyhow::Result;
use std::io::{BufReader, Read, Write};

#[derive(Debug, Clone)]
/// Wrapper for compression information
struct CompressionScheme {
    /// The type of compression that is being used
    pub compression_info: String,
}

impl CompressionScheme {
    /// Creates a new `CompressionScheme` struct, specifying the ZSTD compression algorithm
    pub fn new_lz4_flex() -> Self {
        CompressionScheme {
            compression_info: String::from("LZ4_FLEX"),
        }
    }

    /// Encode a file using the compression algorithm specified in the `CompressionScheme` struct
    pub fn encode<R, W>(&self, source: &mut R, destination: W) -> Result<(), std::io::Error>
    where
        R: std::io::Read,
        W: std::io::Write,
    {
        match self.compression_info.as_str() {
            "LZ4_FLEX" => {
                let mut dest = lz4_flex::frame::FrameEncoder::new(destination);
                std::io::copy(source, &mut dest)?;
                dest.finish().unwrap();
                Ok(())
            }
            _ => panic!("unsupported compression algorithm!"),
        }
    }

    /// Decode a file using the compression algorithm specified in the `CompressionScheme` struct
    pub fn decode<R, W>(&self, source: R, destination: &mut W) -> Result<(), std::io::Error>
    where
        R: std::io::Read,
        W: std::io::Write,
    {
        match self.compression_info.as_str() {
            "LZ4_FLEX" => {
                let mut source = lz4_flex::frame::FrameDecoder::new(source);
                std::io::copy(&mut source, destination)?;
                Ok(())
            }
            _ => panic!("unsupported compression algorithm!"),
        }
    }
}

/// Compresses bytes
pub fn compress_bytes<R, W>(mut reader: R, writer: W) -> Result<()>
where
    R: Read,
    W: Write,
{
    Ok(CompressionScheme::new_lz4_flex().encode(&mut reader, writer)?)
}

/// Decompresses bytes
pub fn decompress_bytes<R, W>(reader: R, mut writer: W) -> Result<()>
where
    R: Read,
    W: Write,
{
    Ok(CompressionScheme::new_lz4_flex().decode(reader, &mut writer)?)
}

#[cfg(not(target_arch = "wasm32"))]
/// Compress the contents of a file at a given path
pub fn compress_file(path: &std::path::Path) -> Result<Vec<u8>> {
    println!("compressing file! {}", path.display());
    // Open the original file (just the first one!)
    let file = std::fs::File::open(path)?;
    // Create a reader for the original file
    let reader = BufReader::new(file);
    // Create a buffer to hold the compressed bytes
    let mut compressed: Vec<u8> = vec![];
    // Compress the chunk before feeding it to WNFS
    compress_bytes(reader, &mut compressed)?;
    // Return compressed bytes
    Ok(compressed)
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

/// Decompress a vector of bytes
pub fn decompress_vec(buf: &[u8]) -> Result<Vec<u8>> {
    // Create a reader for the original file
    let reader = BufReader::new(buf);
    // Create a buffer to hold the compressed bytes
    let mut decompressed: Vec<u8> = vec![];
    // Compress the chunk before feeding it to WNFS
    decompress_bytes(reader, &mut decompressed)?;
    // Return compressed bytes
    Ok(decompressed)
}
