use wnfs::{common::blockstore::BlockStore as WnfsBlockStore, libipld::Cid};

pub trait RootedBlockStore: WnfsBlockStore {
    fn get_root(&self) -> Option<Cid>;
    fn set_root(&self, root: &Cid);
}
