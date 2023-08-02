use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom, Write},
    str::FromStr,
};
use wnfs::libipld::Cid;

use crate::types::{
    blockstore::car::{
        carv1::block::Block,
        carv2::index::{fixture::binary_cid_to_base58_cid, indexbucket::IndexBucket},
        error::CARError,
        varint::*,
    },
    streamable::Streamable,
};

/// Buckets contain a list of values
/// | width (uint32) | count (uint64) | digest1 | digest1 offset (uint64) | digest2 | digest2 offset (uint64) ...
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Bucket {
    pub(crate) cid_width: u32,
    pub(crate) map: HashMap<Cid, u64>,
}

impl Streamable for Bucket {
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self> {
        let start = r.stream_position()?;
        println!("starting bucket read at {} w stream len {}", start, r.stream_len()?);
        // Width of each digest offset pair
        let width = read_varint_u32_exact(r)?;
        // Count of digests
        let count = read_varint_u64_exact(r)?;
        println!("width: {}, count: {}", width, count);

        // Construct a new HashMap
        let mut map = <HashMap<Cid, u64>>::new();
        // While we're successfully able to read in CIDs and offfsets
        let mut counter = count;
        while counter > 0 {
            println!("i am doing this for the {}th time", counter);
            // Read CID bytes
            let cid_start = r.stream_position()?;
            let cid = if let Ok(cid) = Cid::read_bytes(&mut *r) {
                cid
            } else {
                r.seek(SeekFrom::Start(cid_start + 1))?;
                Cid::read_bytes(&mut *r)?
            };
            let offset = read_varint_u64_exact(&mut *r)?;
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
            Ok(Bucket {
                cid_width: width - 8,
                map,
            })
        }
    }

    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<()> {
        println!("starting indexsorted write bytes");
        println!("cid_width is {} and map is {:?}", self.cid_width, self.map);
        w.write_all(&(self.cid_width + 8).to_le_bytes())?;
        w.write_all(&(self.map.len() as u64).to_le_bytes())?;

        // For each cid offset pairing
        for (cid, offset) in self.map.iter() {
            w.write_all(&cid.to_bytes())?;
            w.write_all(&offset.to_le_bytes())?;
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
        if cid.to_bytes().len() as u32 != self.cid_width {
            println!(
                "tried to insert a cid into the wrong bucket w width {}",
                cid.to_bytes().len()
            );
            None
        } else {
            self.map.insert(*cid, offset)
        }
    }
}

impl Bucket {
    // Assumes CIDv1
    // pub(crate) fn new() -> Self {
    //     Bucket {
    //         width: Cid::default().to_bytes().len() as u32,
    //         map: HashMap::new(),
    //     }
    // }

    // pub(crate) fn read_from_carv1<R: Read + Seek>(r: &mut R) -> Result<Self> {
    //     let mut map = HashMap::<Cid, u64>::new();
    //     // While we're able to peek varints and CIDs
    //     while let Ok(block_offset) = r.stream_position() &&
    //           let Ok((varint, cid)) = Block::start_read(&mut *r) {
    //         // Log where we found this block
    //         map.insert(cid, block_offset);

    //         // Skip the rest of the block
    //         r.seek(SeekFrom::Current(varint as i64 - cid.to_bytes().len() as i64))?;
    //     }

    //     let bucket = Bucket {
    //         cid_width: 40,
    //         map,
    //     };

    //     // println!("read_from_carv1 bucket: {:?}", bucket);

    //     Ok(bucket)
    // }
}
