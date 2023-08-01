use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom, Write}, str::FromStr,
};
use wnfs::libipld::Cid;

use crate::types::{
    blockstore::car::{carv1::block::Block, carv2::index::{indexbucket::IndexBucket, fixture::binary_cid_to_base58_cid}, varint::*, error::CARError},
    streamable::Streamable,
};

/// Buckets contain a list of values
/// | width (uint32) | count (uint64) | digest1 | digest1 offset (uint64) | digest2 | digest2 offset (uint64) ...
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Bucket {
    pub(crate) width: u32,
    pub(crate) map: HashMap<Cid, u64>,
}

impl Streamable for Bucket {
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self> {
        let start = r.stream_position()?;
        println!("starting bucket read at {}", start);
        // Width of each digest offset pair
        let width = read_varint_u32_exact(r)?;
        // Count of digests
        let mut count = read_varint_u64_exact(r)?;
        println!("width: {}, count: {}", width, count);

        // Construct a new HashMap
        let mut map = <HashMap<Cid, u64>>::new();
        // While we're successfully able to read in CIDs and offfsets

        let mut cid_bytes = vec![0u8; (width - 8) as usize];

        let mut counter = count;
        while counter > 0 {
            println!("i am doing this for the {}th time", count);
            // Read CID bytes
            r.read_exact(&mut cid_bytes)?;
            println!("successfully read cid_bytes! {:?}", cid_bytes);
            let cid_string = binary_cid_to_base58_cid(&cid_bytes);
            println!("successfully read cid_string! {:?}", cid_string);
            let cid = Cid::from_str(&cid_string)?;
            println!("successfully read cid! {}", cid);
            let offset = read_varint_u64_exact(&mut *r)?;
            println!("successfully read offset! {}", offset);
            map.insert(cid, offset);
            // Decrement
            counter -= 1;   
        }

        // If we failed to read in the correct number of blocks, or there were none at all
        if map.len() as u64 != count || map.len() == 0 {
            // Unread these remaining bytes
            r.seek(SeekFrom::Start(start))?;
            // This is not a bucket
            Err(CARError::EndOfData.into())
        } 
        // Otherwise that was fine
        else {
            Ok(Bucket { width, map })
        }
    }

    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<()> {
        println!("starting indexsorted write bytes");
        let width = 40u32.to_le_bytes();
        let count = encode_varint_u64_exact(self.map.len() as u64);

        w.write_all(&width)?;
        w.write_all(&count)?;

        // For each cid offset pairing
        for (cid, offset) in self.map.iter() {
            w.write_all(&cid.to_bytes())?;
            w.write_all(&encode_varint_u64_exact(*offset))?;
        }

        println!("finished indexsorted write bytes");
        Ok(())
    }
}

impl IndexBucket for Bucket {
    fn get_offset(&self, cid: &Cid) -> Option<u64> {
        self.map.get(cid).copied()
    }

    fn insert_offset(&mut self, cid: &Cid, offset: u64) -> Option<u64> {
        self.map.insert(*cid, offset)
    }
}

impl Bucket {
    // Assumes CIDv1
    pub(crate) fn new() -> Self {
        Bucket {
            width: 40,
            map: HashMap::new(),
        }
    }
}
