use std::fmt::Debug;
use anyhow::Result;
use wnfs::libipld::Cid;
use crate::types::streamable::Streamable;

pub trait IndexBucket: Debug + Streamable {
    fn get_offset(&self, cid: Cid) -> Result<u64>;
    fn insert_offset(&self, cid: Cid, offset: u64) -> Result<()>;
}