use std::io::{Read, Seek, Write};

use crate::types::{
    blockstore::car::{
        carv2::index::{indexbucket::IndexBucket, indexsorted::Bucket as IndexSortedBucket},
        varint::*,
    },
    streamable::Streamable,
};
use anyhow::Result;
use serde::{Serialize, Deserialize};
use wnfs::libipld::Cid;

// | multihash-code (uint64) | width (uint32) | count (uint64) | digest1 | digest1 offset (uint64) | digest2 | digest2 offset (uint64) ...
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub(crate) struct Bucket {
    pub(crate) code: u64,
    pub(crate) bucket: IndexSortedBucket,
}

impl Streamable for Bucket {
    fn read_bytes<R: Read + Seek>(r: &mut R) -> Result<Self> {
        // MultiHash Code
        let code = read_varint_u64(r)?;
        // IndexSorted Bucket
        let bucket = IndexSortedBucket::read_bytes(r)?;
        // Ok
        Ok(Bucket { code, bucket })
    }

    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<()> {
        w.write_all(&encode_varint_u64(self.code))?;
        self.bucket.write_bytes(w)
    }
}

impl IndexBucket for Bucket {
    fn get_offset(&self, cid: &Cid) -> Option<u64> {
        self.bucket.get_offset(cid)
    }

    fn insert_offset(&mut self, cid: &Cid, offset: u64) -> Option<u64> {
        self.bucket.insert_offset(cid, offset)
    }
}

impl Bucket {
    pub(crate) fn new() -> Self {
        // CIDV1
        Bucket { code: 1, bucket: IndexSortedBucket::new() }
    }
}