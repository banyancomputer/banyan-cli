// use anyhow::Result;
use async_trait::async_trait;
use js_sys::Uint8Array;
use std::{io::Cursor, borrow::Cow};
use tomb_common::types::blockstore::car::{carv2::Car, carblockstore::CarBlockStore};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};
use wnfs::{common::blockstore::BlockStore as WnfsBlockStore, libipld::{Cid, IpldCodec}};
use crate::fetch::http::*;

#[wasm_bindgen]
struct WasmBlockStore {
    data: Vec<u8>,
    car: Car
}

#[wasm_bindgen]
impl WasmBlockStore {
    #[wasm_bindgen]
    pub async fn new(url: String) -> Result<WasmBlockStore, JsValue> {
        if let Ok(mut stream) = get_stream(url).await {
            let mut reader = stream.get_reader();
            let mut data: Vec<u8> = vec![];
            while let Ok(Some(result)) = reader.read().await {
                let chunk = Uint8Array::from(result);
                data.extend(chunk.to_vec());
            }

            // Construct CARv2
            let car = Car::read_bytes(Cursor::new(&data)).unwrap();

            Ok(Self {
                data,
                car
            })
        }
        else {
            todo!()
        }
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

impl CarBlockStore for WasmBlockStore {
    fn get_root(&self) -> Option<Cid> {
        self.car.get_root()
    }

    fn set_root(&self, _: &Cid) {
        panic!("WASM BlockStores are read-only")
    }
}

#[cfg(test)]
mod tests {
    use wasm_bindgen_test::wasm_bindgen_test_configure;
    use wasm_bindgen_test::*;
    use crate::metadata::blockstore::WasmBlockStore;
    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn test_load_car() {
        let url = "https://raw.githubusercontent.com/ipld/go-car/master/v2/testdata/sample-v2-indexless.car".to_string();
        assert!(WasmBlockStore::new(url).await.is_ok());
    }
}