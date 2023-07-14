use anyhow::Result;
use async_trait::async_trait;
use wnfs::{
    common::blockstore::BlockStore as WnfsBlockStore,
    libipld::{Cid, IpldCodec},
};

#[async_trait(?Send)]
pub trait TombBlockStore: WnfsBlockStore {
    fn get_root(&self) -> Option<Cid>;
    fn set_root(&self, root: &Cid);
    async fn update_content(&self, cid: &Cid, bytes: Vec<u8>, codec: IpldCodec) -> Result<Cid>;
}
