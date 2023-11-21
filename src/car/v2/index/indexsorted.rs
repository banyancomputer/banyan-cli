use crate::{
    car::{error::CarError, v2::index::indexable::Indexable, Streamable},
    utils::varint::{read_leu32, read_leu64},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom, Write},
};
use wnfs::libipld::Cid;

/// Buckets contain a list of values
/// | width (uint32) | count (uint64) | digest1 | digest1 offset (uint64) | digest2 | digest2 offset (uint64) ...
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Bucket {
    pub(crate) cid_width: u32,
    pub(crate) map: HashMap<Cid, u64>,
}

impl Streamable for Bucket {
    type StreamError = CarError;
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self, Self::StreamError> {
        // Start pos
        let start = r.stream_position()?;
        // Width of each digest offset pair
        let width = read_leu32(r)?;
        // Count of digests
        let count = read_leu64(r)?;

        // Construct a new HashMap
        let mut map = <HashMap<Cid, u64>>::new();
        // While we're successfully able to read in CIDs and offfsets
        let mut counter = count;
        while counter > 0 {
            // Read CID bytes
            let cid_start = r.stream_position()?;
            let cid = if let Ok(cid) = Cid::read_bytes(&mut *r) {
                cid
            } else {
                r.seek(SeekFrom::Start(cid_start + 1))?;
                Cid::read_bytes(&mut *r)?
            };
            let offset = read_leu64(&mut *r)?;
            map.insert(cid, offset);
            // Decrement
            counter -= 1;
        }

        // If we failed to read in the correct number of blocks, or there were none at all
        if map.len() as u64 != count || map.is_empty() {
            // Unread these remaining bytes
            r.seek(SeekFrom::Start(start))?;
            // This is not a bucket
            Err(CarError::end_of_data())
        }
        // Otherwise that was fine
        else {
            Ok(Bucket {
                cid_width: width - 8,
                map,
            })
        }
    }

    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<(), Self::StreamError> {
        w.write_all(&(self.cid_width + 8).to_le_bytes())?;
        w.write_all(&(self.map.len() as u64).to_le_bytes())?;
        // For each cid offset pairing
        for (cid, offset) in self.map.iter() {
            w.write_all(&cid.to_bytes())?;
            w.write_all(&offset.to_le_bytes())?;
        }
        Ok(())
    }
}

impl Indexable for Bucket {
    fn get_offset(&self, cid: &Cid) -> Option<u64> {
        self.map.get(cid).copied()
    }

    fn insert_offset(&mut self, cid: &Cid, offset: u64) -> Option<u64> {
        if cid.to_bytes().len() as u32 != self.cid_width {
            None
        } else {
            self.map.insert(*cid, offset)
        }
    }
}
