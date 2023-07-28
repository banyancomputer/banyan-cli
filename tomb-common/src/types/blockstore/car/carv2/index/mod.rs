pub mod indexbucket;
pub mod indexsorted;
pub mod multihashindexsorted;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use wnfs::libipld::Cid;
use std::{
    fmt::Debug,
    io::{Read, Seek, Write},
};

use crate::types::{
    blockstore::car::{varint::{encode_varint_u128, read_varint_u128}, error::CARError},
    streamable::Streamable,
};
use indexsorted::Bucket as IndexSortedBucket;
use multihashindexsorted::Bucket as MultiHashIndexSortedBucket;

use self::indexbucket::IndexBucket;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Index<I: IndexBucket> {
    pub(crate) codec: u128,
    pub(crate) buckets: Vec<I>
}

/// The Codec associated with the IndexSorted index format 
pub const INDEX_SORTED_CODEC: u128 = 0x0400;
/// The Codec associated with the MultihashIndexSorted index format 
pub const MULTIHASH_INDEX_SORTED_CODEC: u128 = 0x0401;

impl Streamable for Index<IndexSortedBucket> {
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self> {
        // Grab the codec
        let codec = read_varint_u128(r).expect("Cant read varint from stream");
        if codec != INDEX_SORTED_CODEC {
            return Err(CARError::Codec.into())
        }
        // Empty bucket vec
        let mut buckets = <Vec<IndexSortedBucket>>::new();
        
        println!("this file is indexsorted");
        
        // While we can read buckets
        while let Ok(bucket) = IndexSortedBucket::read_bytes(r) {
            // Push new bucket to list
            buckets.push(bucket);
        }

        Ok(Index { codec, buckets })
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

impl Streamable for Index<MultiHashIndexSortedBucket> {
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self> {
        // Grab the codec
        let codec = read_varint_u128(r).expect("Cant read varint from stream");
        if codec != MULTIHASH_INDEX_SORTED_CODEC {
            return Err(CARError::Codec.into())
        }

        // Empty bucket vec
        let mut buckets = <Vec<MultiHashIndexSortedBucket>>::new();

        println!("this file is multihashindexsorted");
        // While we can read buckets
        while let Ok(bucket) = MultiHashIndexSortedBucket::read_bytes(r) {
            // Push new bucket to list
            buckets.push(bucket);
        }

        Ok(Index { codec, buckets })
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

impl Index<MultiHashIndexSortedBucket> {
    pub(crate) fn new() -> Self {
        Index {
            codec: MULTIHASH_INDEX_SORTED_CODEC,
            buckets: vec![MultiHashIndexSortedBucket::new()]
        }
    }
}

impl Index<IndexSortedBucket> {
    pub(crate) fn new() -> Self {
        Index {
            codec: MULTIHASH_INDEX_SORTED_CODEC,
            buckets: vec![IndexSortedBucket::new()]
        }
    }
}

impl IndexBucket for Index<MultiHashIndexSortedBucket> {
    fn get_offset(&self, cid: &Cid) -> Option<u64> {
        self.buckets[0].get_offset(cid)
    }

    fn insert_offset(&mut self, cid: &Cid, offset: u64) -> Option<u64> {
        self.buckets[0].insert_offset(cid, offset)
    }
}

impl IndexBucket for Index<IndexSortedBucket> {
    fn get_offset(&self, cid: &Cid) -> Option<u64> {
        self.buckets[0].get_offset(cid)
    }

    fn insert_offset(&mut self, cid: &Cid, offset: u64) -> Option<u64> {
        self.buckets[0].insert_offset(cid, offset)
    }
}
#[cfg(test)]
mod test {
    use anyhow::Result;
    use wnfs::libipld::Cid;
    use crate::types::blockstore::car::carv2::index::multihashindexsorted::Bucket;

    use super::{Index, indexbucket::IndexBucket};

    #[test]
    fn insert_retrieve() -> Result<()> {
        // Create a new v2 index
        let mut index = <Index<Bucket>>::new();
        // Put a new cid in
        index.insert_offset(&Cid::default(), 42);
        let offset = index.get_offset(&Cid::default());
        assert_eq!(offset, Some(42));
        Ok(())
    }


    // TODO: Until valid fixtures can be made or obtained this is a waste of time
    /*
    #[test]
    #[serial]
    #[ignore]
    fn read_multihashindex() -> Result<()> {
        // This fixture uses the multihash index sorted CARv2 Index
        let index_path = car_setup(2, "basic-index", "read_multihashindex")?;
        let rw = &mut get_read_write(&index_path)?;
        let car = CAR::read_bytes(rw)?;

        Ok(())
    }

    #[test]
    #[serial]
    #[ignore]
    fn read_multihashcar() -> Result<()> {
        // This fixture uses the multihash index sorted CARv2 Index
        let index_path = car_setup(2, "rw-bs", "read_multihashcar")?;
        let rw = &mut get_read_write(&index_path)?;
        let car = CAR::read_bytes(rw)?;

        Ok(())
    }

    #[test]
    #[serial]
    #[ignore]
    fn read_sortedindexcar() -> Result<()> {
        // This fixture uses the multihash index sorted CARv2 Index
        let index_path = car_setup(2, "rw-bs", "read_sortedindexcar")?;
        let rw = &mut get_read_write(&index_path)?;
        let car = CAR::read_bytes(rw)?;

        Ok(())
    }
    */
}
