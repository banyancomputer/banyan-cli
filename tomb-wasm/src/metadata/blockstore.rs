// use anyhow::Result;
use crate::fetch::http::*;
use async_trait::async_trait;
use std::{borrow::Cow, io::Cursor};
use tomb_common::types::blockstore::{car::carv2::CAR, tombblockstore::TombBlockStore};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use wnfs::{
    common::blockstore::BlockStore as WnfsBlockStore,
    libipld::{Cid, IpldCodec},
};

/// CARv2 MemoryBlockStore in WASM
#[wasm_bindgen]
#[derive(Debug)]
pub struct WasmBlockStore {
    data: Vec<u8>,
    car: CAR,
}

#[allow(dead_code)]
#[wasm_bindgen]
impl WasmBlockStore {
    #[wasm_bindgen]
    pub async fn new(url: String) -> Result<WasmBlockStore, JsValue> {
        // Load data
        let data = get_data(url.clone()).await.unwrap();
        // Load car
        let car = CAR::read_bytes(Cursor::new(&data)).unwrap();
        // Ok
        Ok(Self { data, car })
    }
}

#[async_trait(?Send)]
impl WnfsBlockStore for WasmBlockStore {
    async fn get_block(&self, cid: &Cid) -> Result<Cow<'_, Vec<u8>>, anyhow::Error> {
        let mut reader = Cursor::new(&self.data);
        let block = self.car.get_block(cid, &mut reader)?;
        Ok(Cow::Owned(block.content))
    }

    async fn put_block(&self, _: Vec<u8>, _: IpldCodec) -> Result<Cid, anyhow::Error> {
        panic!("WASM BlockStores are read-only")
    }
}

#[async_trait(?Send)]
impl TombBlockStore for WasmBlockStore {
    fn get_root(&self) -> Option<Cid> {
        self.car.get_root()
    }

    fn set_root(&self, _: &Cid) {
        panic!("WASM BlockStores are read-only")
    }

    async fn update_content(&self, _: &Cid, _: Vec<u8>, _: IpldCodec) -> anyhow::Result<Cid> {
        panic!("WASM BlockStores are read-only")
    }
}

#[cfg(test)]
mod test {
    use crate::metadata::blockstore::WasmBlockStore;
    use wasm_bindgen_test::wasm_bindgen_test_configure;
    use wasm_bindgen_test::*;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_load_car() {
        let url = "https://raw.githubusercontent.com/ipld/go-car/master/v2/testdata/sample-v2-indexless.car".to_string();
        assert!(WasmBlockStore::new(url).await.is_ok());
    }
}
