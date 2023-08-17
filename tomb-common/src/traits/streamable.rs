use anyhow::Result;
use std::io::{Read, Seek, Write};

/// Custom Stream-Based Serialization
pub trait Streamable: Sized {
    /// Read the bytes
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self>;
    /// Write the bytes
    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<()>;
}