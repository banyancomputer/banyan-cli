use crate::types::streamable::Streamable;
use anyhow::Result;
use std::fmt::Debug;
use wnfs::libipld::Cid;

pub trait IndexBucket: Debug + Streamable {
    fn get_offset(&self, cid: Cid) -> Result<u64>;
    fn insert_offset(&self, cid: Cid, offset: u64) -> Result<()>;
}
