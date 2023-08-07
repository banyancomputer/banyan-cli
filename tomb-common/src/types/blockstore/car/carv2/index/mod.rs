/// The trait that describes bucket formats internal to the Index
pub mod indexable;
/// The simple Bucket format
pub mod indexsorted;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Debug,
    io::{Read, Seek, SeekFrom, Write},
};
use wnfs::libipld::Cid;

use self::indexable::Indexable;
use crate::types::{
    blockstore::car::{
        carv1::block::Block,
        error::CARError,
        varint::{encode_varint_u128, read_varint_u128},
    },
    streamable::Streamable,
};
use indexsorted::Bucket;

/// The type of Index requires a format, and contains both a codec and a Bucket vec
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Index<I: Indexable> {
    pub(crate) codec: u128,
    pub(crate) buckets: Vec<I>,
}

/// The Codec associated with the IndexSorted index format
pub const INDEX_SORTED_CODEC: u128 = 0x0400;
/// The Codec associated with the MultihashIndexSorted index format
pub const MULTIHASH_INDEX_SORTED_CODEC: u128 = 0x0401;

impl Streamable for Index<Bucket> {
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self> {
        // Grab the codec
        let codec = read_varint_u128(r).expect("Cant read varint from stream");
        if codec != INDEX_SORTED_CODEC {
            return Err(CARError::Codec.into());
        }
        // Empty bucket vec
        let mut buckets = <Vec<Bucket>>::new();
        // While we can read buckets
        while let Ok(bucket) = Bucket::read_bytes(r) {
            // Push new bucket to list
            buckets.push(bucket);
        }

        // If there are no buckets
        if buckets.is_empty() {
            // At least start out with an empty one
            Err(CARError::Index.into())
        } else {
            // Success
            Ok(Index { codec, buckets })
        }
    }

    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<()> {
        // Write codec
        w.write_all(&encode_varint_u128(self.codec))?;
        // For each bucket
        for bucket in &self.buckets {
            // Write out
            bucket.write_bytes(w)?;
        }
        Ok(())
    }
}

impl Indexable for Index<Bucket> {
    fn get_offset(&self, cid: &Cid) -> Option<u64> {
        for bucket in &self.buckets {
            if let Some(offset) = bucket.get_offset(cid) {
                return Some(offset);
            }
        }

        None
    }

    fn insert_offset(&mut self, cid: &Cid, offset: u64) -> Option<u64> {
        let cid_width = cid.to_bytes().len() as u32;

        for bucket in &mut self.buckets {
            if bucket.cid_width == cid_width {
                return bucket.insert_offset(cid, offset);
            }
        }

        let mut new_map = HashMap::new();
        new_map.insert(*cid, offset);
        self.buckets.push(Bucket {
            cid_width,
            map: new_map,
        });
        None
    }
}

impl Index<Bucket> {
    pub(crate) fn read_from_carv1<R: Read + Seek>(r: &mut R) -> Result<Self> {
        let mut new_index: Index<Bucket> = Index {
            codec: INDEX_SORTED_CODEC,
            buckets: vec![],
        };

        // While we're able to peek varints and CIDs
        while let Ok(block_offset) = r.stream_position() &&
              let Ok((varint, cid)) = Block::start_read(&mut *r) {
            // Log where we found this block
            new_index.insert_offset(&cid, block_offset);
            // Skip the rest of the block
            r.seek(SeekFrom::Current(varint as i64 - cid.to_bytes().len() as i64))?;
        }

        Ok(new_index)
    }

    /// Accumulate a vec of all Cids in all Buckets
    pub fn get_all_cids(&self) -> Vec<Cid> {
        let mut cids = <Vec<Cid>>::new();
        for bucket in self.buckets.clone() {
            cids.extend_from_slice(&bucket.map.into_keys().collect::<Vec<Cid>>())
        }
        cids.sort();
        cids
    }
}

mod test {
    use super::{Bucket, Index, INDEX_SORTED_CODEC};
    use crate::streamable_tests;
    use std::{collections::HashMap, str::FromStr};
    use wnfs::libipld::Cid;

    /// Generate example data for Bucket
    #[allow(dead_code)]
    fn index_sorted_example() -> Bucket {
        let cid = Cid::from_str("bafyrcfajghwtmjky5lzbkwxyzjlim3yxi4pmebi").unwrap();
        // Width represents
        let cid_width = cid.to_bytes().len() as u32;
        let mut map = HashMap::new();
        map.insert(cid, 42);
        Bucket { cid_width, map }
    }

    /// Generate example data for V2Index
    #[allow(dead_code)]
    fn v2_sorted_index_example() -> Index<Bucket> {
        Index {
            codec: INDEX_SORTED_CODEC,
            buckets: vec![index_sorted_example()],
        }
    }

    streamable_tests! {
        Bucket:
        indexsorted: index_sorted_example(),

        Index<Bucket>:
        carv2sortedindex: v2_sorted_index_example(),
    }
}
