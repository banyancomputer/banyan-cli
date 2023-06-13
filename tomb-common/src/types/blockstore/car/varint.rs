use anyhow::Result;
use std::io::{Read, Seek, SeekFrom};
use unsigned_varint::{decode, encode};

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
