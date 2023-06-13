use anyhow::Result;
use std::io::{Read, Seek, SeekFrom};
use unsigned_varint::{decode, encode};

pub(crate) fn read_varint_u32<R: Read + Seek>(r: &mut R) -> Result<u32> {
    // Create buffer
    let mut buf = encode::u32_buffer();
    // Read from stream
    r.read_exact(&mut buf)?;
    // Decode
    let (result, remaining) = decode::u32(&buf)?;
    // Rewind
    r.seek(SeekFrom::Current(-(remaining.len() as i64)))?;
    // Ok
    Ok(result)
}

pub(crate) fn read_varint_u64<R: Read + Seek>(r: &mut R) -> Result<u64> {
    // Create buffer
    let mut buf = encode::u64_buffer();
    // Read from stream
    r.read_exact(&mut buf)?;
    // Decode
    let (result, remaining) = decode::u64(&buf)?;
    // Rewind
    r.seek(SeekFrom::Current(-(remaining.len() as i64)))?;
    // Ok
    Ok(result)
}

pub(crate) fn read_varint_u128<R: Read + Seek>(r: &mut R) -> Result<u128> {
    // Create buffer
    let mut buf = encode::u128_buffer();
    // Read from stream
    r.read_exact(&mut buf)?;
    // Decode
    let (result, remaining) = decode::u128(&buf)?;
    // Rewind
    r.seek(SeekFrom::Current(-(remaining.len() as i64)))?;
    // Ok
    Ok(result)
}

pub(crate) fn read_varint_u32_exact<R: Read>(r: &mut R) -> Result<u32> {
    // Create and fill buffer
    let mut buf: [u8; 4] = [0; 4];
    r.read_exact(&mut buf)?;
    // Decode little endian
    Ok(u32::from_le_bytes(buf))
}

pub(crate) fn read_varint_u64_exact<R: Read>(r: &mut R) -> Result<u64> {
    // Create and fill buffer
    let mut buf: [u8; 8] = [0; 8];
    r.read_exact(&mut buf)?;
    // Decode little endian
    Ok(u64::from_le_bytes(buf))
}

pub(crate) fn read_varint_u128_exact<R: Read>(r: &mut R) -> Result<u128> {
    // Create and fill buffer
    let mut buf: [u8; 16] = [0; 16];
    r.read_exact(&mut buf)?;
    // Decode little endian
    Ok(u128::from_le_bytes(buf))
}

pub(crate) fn encode_varint_u32(input: u32) -> Vec<u8> {
    // Create buffer
    let mut buf = encode::u32_buffer();
    // Encode bytes
    encode::u32(input, &mut buf).to_vec()
}

pub(crate) fn encode_varint_u64(input: u64) -> Vec<u8> {
    // Create buffer
    let mut buf = encode::u64_buffer();
    // Encode bytes
    encode::u64(input, &mut buf).to_vec()
}

pub(crate) fn encode_varint_u128(input: u128) -> Vec<u8> {
    // Create buffer
    let mut buf = encode::u128_buffer();
    // Encode bytes
    encode::u128(input, &mut buf).to_vec()
}

pub(crate) fn encode_varint_u64_exact(input: u64) -> [u8; 8] {
    input.to_le_bytes()
}

pub(crate) fn encode_varint_u128_exact(input: u128) -> [u8; 16] {
    input.to_le_bytes()
}
