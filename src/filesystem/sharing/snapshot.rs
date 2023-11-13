use std::collections::BTreeSet;
use wnfs::libipld::Cid;

pub struct SharedFile {
    cids: BTreeSet<Cid>,
}
