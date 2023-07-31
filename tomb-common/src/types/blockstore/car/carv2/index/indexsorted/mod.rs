use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom, Write},
};
use wnfs::libipld::Cid;

use crate::types::{
    blockstore::car::{carv1::block::Block, carv2::index::indexbucket::IndexBucket, varint::*},
    streamable::Streamable,
};

/// Buckets contain a list of values
/// | width (uint32) | count (uint64) | digest1 | digest1 offset (uint64) | digest2 | digest2 offset (uint64) ...
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Bucket {
    pub(crate) width: u32,
    pub(crate) count: u64,
    pub(crate) map: HashMap<Cid, u64>,
}

impl Streamable for Bucket {
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self> {
        // Width of the Bucket
        let width = read_varint_u32(r)?;
        // Count of digests
        let count = read_varint_u64(r)?;

        // Construct a new HashMap
        let mut map = <HashMap<Cid, u64>>::new();
        // While we're successfully able to read in CIDs and offfsets
        while let Ok(cid) = Cid::read_bytes(&mut *r) &&
              let Ok(offset) = read_varint_u64(&mut *r) {
            // Insert those offfsets into the map
            map.insert(cid, offset);
        }

        // Ok
        Ok(Bucket { width, count, map })
    }

    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<()> {
        let width = encode_varint_u32(self.width);
        let count = encode_varint_u64(self.count);

        w.write_all(&width)?;
        w.write_all(&count)?;

        // For each cid offset pairing
        for (cid, offset) in self.map.iter() {
            w.write_all(&cid.to_bytes())?;
            w.write_all(&encode_varint_u64(*offset))?;
        }

        Ok(())
    }
}

impl IndexBucket for Bucket {
    fn get_offset(&self, cid: &Cid) -> Option<u64> {
        self.map.get(cid).copied()
    }

    fn insert_offset(&mut self, cid: &Cid, offset: u64) -> Option<u64> {
        self.count += 1;
        self.map.insert(*cid, offset)
    }
}

impl Bucket {
    // Assumes CIDv1
    pub(crate) fn new() -> Self {
        Bucket {
            width: 40,
            count: 0,
            map: HashMap::new(),
        }
    }

    pub(crate) fn read_from_carv1<R: Read + Seek>(r: &mut R) -> Result<Self> {
        let mut map = HashMap::<Cid, u64>::new();
        // While we're able to peek varints and CIDs
        while let Ok(block_offset) = r.stream_position() &&
              let Ok((varint, cid)) = Block::start_read(&mut *r) {
            // Log where we found this block
            map.insert(cid, block_offset);
            // Skip the rest of the block
            r.seek(SeekFrom::Current(varint as i64 - cid.to_bytes().len() as i64))?;
        }

        let bucket = Bucket {
            width: 40,
            count: map.len() as u64,
            map,
        };

        // println!("read_from_carv1 bucket: {:?}", bucket);

        Ok(bucket)
    }
}
