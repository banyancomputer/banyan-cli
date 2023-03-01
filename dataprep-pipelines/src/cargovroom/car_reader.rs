use crate::types::pipeline::CarsWriterLocation;
use anyhow::Result;
use cid::multihash::MultihashDigest;
use cid::{multihash, Cid};
use integer_encoding::VarIntReader;
use ipld::codec::Decode;
use ipld_cbor::DagCborCodec;
use std::io::{Cursor, Read, Seek};
use tokio::sync::RwLock;

/// get a block of bytes based on a CarsWriterLocation!
pub async fn get_block<R: Seek + Read + Unpin>(
    location: CarsWriterLocation,
    car_reader: RwLock<R>,
) -> Result<Vec<u8>> {
    let mut car_reader = car_reader.write().await;
    car_reader.seek(tokio::io::SeekFrom::Start(location.offset as u64))?;
    let len: usize = VarIntReader::read_varint(&mut *car_reader)?;
    println!("decode: len: {}", len);
    // make a buf put the cid
    let mut everything_else_buf = vec![0; len];
    car_reader.read_exact(&mut everything_else_buf)?;
    println!("everything_else_buf first 100: {:?}", &everything_else_buf[0..100]);
    let mut cursor = Cursor::new(&everything_else_buf);
    let cid = Cid::read_bytes(&mut cursor)?;
    println!("decode: cid: {}", cid.to_string());
    let cursor_location = cursor.position() as usize;
    let everything_else_buf = cursor.into_inner()[cursor_location..].to_vec();
    println!("decode: data: {:?}", everything_else_buf);

    // compute the digest of everything_else_buf
    // and compare it to the digest in the cid
    // if they match, return everything_else_buf
    // otherwise, return an error
    let digest = multihash::Code::Sha2_256.digest(&everything_else_buf);
    let cid_computed = Cid::new_v1(DagCborCodec.into(), digest);
    if cid == cid_computed {
        Ok(everything_else_buf)
    } else {
        Err(anyhow::anyhow!("hey!! cid doesn't match block contents!"))
    }
}
