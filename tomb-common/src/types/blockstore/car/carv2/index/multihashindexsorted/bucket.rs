use std::io::{Write, Seek, Read};

use anyhow::Result;
use wnfs::libipld::Cid;
use crate::types::{blockstore::car::{varint::*, carv2::index::{indexsorted::bucket::Bucket as IndexSortedBucket, indexbucket::IndexBucket}}, streamable::Streamable};

// | multihash-code (uint64) | width (uint32) | count (uint64) | digest1 | digest1 offset (uint64) | digest2 | digest2 offset (uint64) ...
#[derive(Debug, PartialEq, Clone, Default)]
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
        Ok(Bucket {
            code,
            bucket,
        })
    }

    fn write_bytes<W: Write + Seek>(&self, w: &mut W) -> Result<()> {
        w.write_all(&encode_varint_u64(self.code))?;
        self.bucket.write_bytes(w)
    }
}

impl IndexBucket for Bucket {
    fn get_offset(&self, cid: Cid) -> Result<u64> {
        todo!()
    }

    fn insert_offset(&self, cid: Cid, offset: u64) -> Result<()> {
        todo!()
    }
}