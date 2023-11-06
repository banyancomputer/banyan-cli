use crate::banyan_common::traits::streamable::Streamable;
use std::fmt::Debug;
use wnfs::libipld::Cid;

/// Special kind of hashmap for dealing with offsets
pub trait Indexable: Debug + Streamable + Send {
    /// Get a CID's offset if it exists
    fn get_offset(&self, cid: &Cid) -> Option<u64>;
    /// Set or update a CID's offset
    fn insert_offset(&mut self, cid: &Cid, offset: u64) -> Option<u64>;
}
