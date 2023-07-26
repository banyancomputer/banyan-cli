use crate::types::streamable::Streamable;
use std::fmt::Debug;
use wnfs::libipld::Cid;

pub trait IndexBucket: Debug + Streamable + Send {
    fn get_offset(&self, cid: &Cid) -> Option<u64>;
    fn insert_offset(&mut self, cid: &Cid, offset: u64) -> Option<u64>;
}
