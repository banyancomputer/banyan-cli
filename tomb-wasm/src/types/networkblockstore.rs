use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::JsFuture;
use wnfs::{
    common::BlockStore as WnfsBlockStore,
    libipld::{Cid, IpldCodec},
};
